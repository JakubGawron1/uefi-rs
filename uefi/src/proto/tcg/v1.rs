//! [TCG] (Trusted Computing Group) protocol for [TPM] (Trusted Platform
//! Module) 1.1 and 1.2.
//!
//! This protocol is defined in the [TCG EFI Protocol Specification _for
//! TPM Family 1.1 or 1.2_][spec].
//!
//! [spec]: https://trustedcomputinggroup.org/resource/tcg-efi-protocol-specification/
//! [TCG]: https://trustedcomputinggroup.org/
//! [TPM]: https://en.wikipedia.org/wiki/Trusted_Platform_Module

use super::{usize_from_u32, EventType, HashAlgorithm, PcrIndex};
use crate::data_types::PhysicalAddress;
use crate::proto::unsafe_protocol;
use crate::{Result, Status};
use core::fmt::{self, Debug, Formatter};
use core::marker::PhantomData;
use core::{mem, ptr};
use ptr_meta::Pointee;

/// 20-byte SHA-1 digest.
pub type Sha1Digest = [u8; 20];

/// Information about the protocol and the TPM device.
///
/// Layout compatible with the C type `TCG_EFI_BOOT_SERVICE_CAPABILITY`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct BootServiceCapability {
    size: u8,
    structure_version: Version,
    protocol_spec_version: Version,
    hash_algorithm_bitmap: u8,
    tpm_present_flag: u8,
    tpm_deactivated_flag: u8,
}

impl BootServiceCapability {
    /// Version of the `BootServiceCapability` structure.
    #[must_use]
    pub fn structure_version(&self) -> Version {
        self.structure_version
    }

    /// Version of the `Tcg` protocol.
    #[must_use]
    pub fn protocol_spec_version(&self) -> Version {
        self.protocol_spec_version
    }

    /// Supported hash algorithms.
    #[must_use]
    pub fn hash_algorithm(&self) -> HashAlgorithm {
        // Safety: the value should always be 0x1 (indicating SHA-1), but
        // we don't care if it's some unexpected value.
        unsafe { HashAlgorithm::from_bits_unchecked(u32::from(self.hash_algorithm_bitmap)) }
    }

    /// Whether the TPM device is present.
    #[must_use]
    pub fn tpm_present(&self) -> bool {
        self.tpm_present_flag != 0
    }

    /// Whether the TPM device is deactivated.
    #[must_use]
    pub fn tpm_deactivated(&self) -> bool {
        self.tpm_deactivated_flag != 0
    }
}

/// Version information.
///
/// Layout compatible with the C type `TCG_VERSION`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Version {
    /// Major version.
    pub major: u8,
    /// Minor version.
    pub minor: u8,

    // Leave these two fields undocumented since it's not clear what
    // they are for. The spec doesn't say, and they were removed in the
    // v2 spec.
    #[allow(missing_docs)]
    pub rev_major: u8,
    #[allow(missing_docs)]
    pub rev_minor: u8,
}

/// Entry in the [`EventLog`].
///
/// Layout compatible with the C type `TCG_PCR_EVENT`.
///
/// Naming note: the spec refers to "event data" in two conflicting
/// ways: the `event_data` field and the data hashed in the digest
/// field. These two are independent; although the event data _can_ be
/// what is hashed in the digest field, it doesn't have to be.
#[repr(C, packed)]
#[derive(Pointee)]
pub struct PcrEvent {
    pcr_index: PcrIndex,
    event_type: EventType,
    digest: Sha1Digest,
    event_data_size: u32,
    event_data: [u8],
}

impl PcrEvent {
    pub(super) unsafe fn from_ptr<'a>(ptr: *const u8) -> &'a Self {
        // Get the `event_size` field.
        let ptr_u32: *const u32 = ptr.cast();
        let event_size = ptr_u32.add(7).read_unaligned();
        let event_size = usize_from_u32(event_size);
        unsafe { &*ptr_meta::from_raw_parts(ptr.cast(), event_size) }
    }

    /// PCR index for the event.
    #[must_use]
    pub fn pcr_index(&self) -> PcrIndex {
        self.pcr_index
    }

    /// Type of event, indicating what type of data is stored in [`event_data`].
    ///
    /// [`event_data`]: Self::event_data
    #[must_use]
    pub fn event_type(&self) -> EventType {
        self.event_type
    }

    /// Raw event data. The meaning of this data can be determined from
    /// the [`event_type`].
    ///
    /// Note that this data is independent of what is hashed [`digest`].
    ///
    /// [`digest`]: Self::digest
    /// [`event_type`]: Self::event_type
    #[must_use]
    pub fn event_data(&self) -> &[u8] {
        &self.event_data
    }

    /// SHA-1 digest of the data hashed for this event.
    #[must_use]
    pub fn digest(&self) -> Sha1Digest {
        self.digest
    }
}

// Manual `Debug` implementation since it can't be derived for a packed DST.
impl Debug for PcrEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PcrEvent")
            .field("pcr_index", &{ self.pcr_index })
            .field("event_type", &{ self.event_type })
            .field("digest", &self.digest)
            .field("event_data_size", &{ self.event_data_size })
            .field("event_data", &&self.event_data)
            .finish()
    }
}

/// TPM event log.
///
/// This type of event log always uses SHA-1 hashes.
///
/// [`v1::Tcg`]: Tcg
/// [`v2::Tcg`]: super::v2::Tcg
pub struct EventLog<'a> {
    // Tie the lifetime to the protocol, and by extension, boot services.
    _lifetime: PhantomData<&'a Tcg>,

    location: *const u8,
    last_entry: *const u8,

    is_truncated: bool,
}

impl<'a> EventLog<'a> {
    pub(super) unsafe fn new(
        location: *const u8,
        last_entry: *const u8,
        is_truncated: bool,
    ) -> Self {
        Self {
            _lifetime: PhantomData,
            location,
            last_entry,
            is_truncated,
        }
    }

    /// Iterator of events in the log.
    #[must_use]
    pub fn iter(&self) -> EventLogIter {
        EventLogIter {
            log: self,
            location: self.location,
        }
    }

    /// If true, the event log is missing one or more entries because
    /// additional events would have exceeded the space allocated for
    /// the log.
    ///
    /// This value is not reported for the [`v1::Tcg`] protocol, so it
    /// is always `false` in that case.
    ///
    /// [`v1::Tcg`]: Tcg
    #[must_use]
    pub fn is_truncated(&self) -> bool {
        self.is_truncated
    }
}

/// Iterator for events in [`EventLog`].
pub struct EventLogIter<'a> {
    log: &'a EventLog<'a>,
    location: *const u8,
}

impl<'a> Iterator for EventLogIter<'a> {
    type Item = &'a PcrEvent;

    fn next(&mut self) -> Option<Self::Item> {
        // The spec says that `last_entry` will be null if there are no
        // events. Presumably `location` will be null as well, but check
        // both just to be safe.
        if self.location.is_null() || self.log.last_entry.is_null() {
            return None;
        }

        // Safety: we trust that the protocol has given us a valid range
        // of memory to read from.
        let event = unsafe { PcrEvent::from_ptr(self.location) };

        // If this is the last entry, set the location to null so that
        // future calls to `next()` return `None`.
        if self.location == self.log.last_entry {
            self.location = ptr::null();
        } else {
            self.location = unsafe { self.location.add(mem::size_of_val(event)) };
        }

        Some(event)
    }
}

/// Protocol for interacting with TPM 1.1 and 1.2 devices.
///
/// The corresponding C type is `EFI_TCG_PROTOCOL`.
#[repr(C)]
#[unsafe_protocol("f541796d-a62e-4954-a775-9584f61b9cdd")]
pub struct Tcg {
    status_check: unsafe extern "efiapi" fn(
        this: *mut Tcg,
        protocol_capability: *mut BootServiceCapability,
        feature_flags: *mut u32,
        event_log_location: *mut PhysicalAddress,
        event_log_last_entry: *mut PhysicalAddress,
    ) -> Status,

    // TODO: fill these in and provide a public interface.
    hash_all: unsafe extern "efiapi" fn() -> Status,
    log_event: unsafe extern "efiapi" fn() -> Status,
    pass_through_to_tpm: unsafe extern "efiapi" fn() -> Status,
    hash_log_extend_event: unsafe extern "efiapi" fn() -> Status,
}

/// Return type of [`Tcg::status_check`].
pub struct StatusCheck<'a> {
    /// Information about the protocol and the TPM device.
    pub protocol_capability: BootServiceCapability,

    /// Feature flags. The spec does not define any feature flags, so
    /// this is always expected to be zero.
    pub feature_flags: u32,

    /// TPM event log.
    pub event_log: EventLog<'a>,
}

impl Tcg {
    /// Get information about the protocol and TPM device, as well as
    /// the TPM event log.
    pub fn status_check(&mut self) -> Result<StatusCheck> {
        let mut protocol_capability = BootServiceCapability::default();
        let mut feature_flags = 0;
        let mut event_log_location = 0;
        let mut event_log_last_entry = 0;

        let status = unsafe {
            (self.status_check)(
                self,
                &mut protocol_capability,
                &mut feature_flags,
                &mut event_log_location,
                &mut event_log_last_entry,
            )
        };

        if status.is_success() {
            // The truncated field is just there for the v2 protocol;
            // always set it to false for v1.
            let truncated = false;
            let event_log = unsafe {
                EventLog::new(
                    event_log_location as *const u8,
                    event_log_last_entry as *const u8,
                    truncated,
                )
            };

            Ok(StatusCheck {
                protocol_capability,
                feature_flags,
                event_log,
            })
        } else {
            Err(status.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_log_v1() {
        // This data comes from dumping the TPM event log in a VM
        // (truncated to just two entries).
        #[rustfmt::skip]
        let bytes = [
            // Event 1
            // PCR index
            0x00, 0x00, 0x00, 0x00,
            // Event type
            0x08, 0x00, 0x00, 0x00,
            // SHA1 digest
            0x14, 0x89, 0xf9, 0x23, 0xc4, 0xdc, 0xa7, 0x29, 0x17, 0x8b,
            0x3e, 0x32, 0x33, 0x45, 0x85, 0x50, 0xd8, 0xdd, 0xdf, 0x29,
            // Event data size
            0x02, 0x00, 0x00, 0x00,
            // Event data
            0x00, 0x00,

            // Event 2
            // PCR index
            0x00, 0x00, 0x00, 0x00,
            // Event type
            0x08, 0x00, 0x00, 0x80,
            // SHA1 digest
            0xc7, 0x06, 0xe7, 0xdd, 0x36, 0x39, 0x29, 0x84, 0xeb, 0x06,
            0xaa, 0xa0, 0x8f, 0xf3, 0x36, 0x84, 0x40, 0x77, 0xb3, 0xed,
            // Event data size
            0x10, 0x00, 0x00, 0x00,
            // Event data
            0x00, 0x00, 0x82, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let log = unsafe { EventLog::new(bytes.as_ptr(), bytes.as_ptr().add(34), false) };
        let mut iter = log.iter();

        // Entry 1
        let entry = iter.next().unwrap();
        assert_eq!(entry.pcr_index(), PcrIndex(0));
        assert_eq!(entry.event_type(), EventType::CRTM_VERSION);
        #[rustfmt::skip]
        assert_eq!(
            entry.digest(),
            [
                0x14, 0x89, 0xf9, 0x23, 0xc4, 0xdc, 0xa7, 0x29, 0x17, 0x8b,
                0x3e, 0x32, 0x33, 0x45, 0x85, 0x50, 0xd8, 0xdd, 0xdf, 0x29,
            ]
        );
        assert_eq!(entry.event_data(), [0x00, 0x00]);

        // Entry 2
        let entry = iter.next().unwrap();
        assert_eq!(entry.pcr_index(), PcrIndex(0));
        assert_eq!(entry.event_type(), EventType::EFI_PLATFORM_FIRMWARE_BLOB);
        #[rustfmt::skip]
        assert_eq!(
            entry.digest(),
            [
                0xc7, 0x06, 0xe7, 0xdd, 0x36, 0x39, 0x29, 0x84, 0xeb, 0x06,
                0xaa, 0xa0, 0x8f, 0xf3, 0x36, 0x84, 0x40, 0x77, 0xb3, 0xed,
            ]
        );
        #[rustfmt::skip]
        assert_eq!(
            entry.event_data(),
            [
                0x00, 0x00, 0x82, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]
        );
    }
}
