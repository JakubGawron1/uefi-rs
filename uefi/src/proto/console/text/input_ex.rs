use crate::proto::unsafe_protocol;
use crate::{Char16, Event, Result, Status};

use core::mem::MaybeUninit;

#[repr(C)]
#[unsafe_protocol("dd9e7534-7762-4698-8c14-f58517a625aa")]
pub struct Input_ex{
    reset: extern "efiapi" fn(this: &mut Input_ex, extended: bool ) -> Status,
    read_key_stroke_ex: extern "efiapi" fn (this: &mut Input_ex, key: *mut RawKey)-> Status,
    wait_for_key: Event,
    
}

impl Input_ex{
    pub fn reset(&mut self, extended_verification: bool) -> Result {
        (self.reset)(self, extended_verification).into()
    }

    pub fn read_key(&mut self) -> Result<Option<Key_ex>> {
        let mut key = MaybeUninit::<RawKey>::uninit();

        match (self.read_key_stroke_ex)(self, key.as_mut_ptr()) {
            Status::NOT_READY => Ok(None),
            other => other.into_with_val(|| Some(unsafe { key.assume_init() }.into())),
        }
    }

    /// Event to be used with `BootServices::wait_for_event()` in order to wait
    /// for a key to be available
    #[must_use]
    pub const fn wait_for_key_event(&self) -> &Event {
        &self.wait_for_key
    }
}

/// A key read from the console (high-level version)
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Key_ex {
    /// The key is associated with a printable Unicode character
    Printable(Char16),

    /// The key is special (arrow, function, multimedia...)
    Special(ScanCode_ex),
}

impl From<RawKey> for Key_ex {
    fn from(k: RawKey) -> Key_ex {
        if k.scan_code == ScanCode_ex::NULL {
            Key_ex::Printable(k.unicode_char)
        } else {
            Key_ex::Special(k.scan_code)
        }
    }
}

/// A key read from the console (UEFI version)
#[repr(C)]
pub struct RawKey {
    /// The key's scan code.
    /// or 0 if printable
    pub scan_code: ScanCode_ex,
    /// Associated Unicode character,
    /// or 0 if not printable.
    pub unicode_char: Char16,
}

newtype_enum! {
/// A keyboard scan code
///
/// Codes 0x8000 -> 0xFFFF are reserved for future OEM extensibility, therefore
/// this C enum is _not_ safe to model as a Rust enum (where the compiler must
/// know about all variants at compile time).
pub enum ScanCode_ex: u16 => #[allow(missing_docs)] {
    /// Null scan code, indicates that the Unicode character should be used.
    NULL        = 0x00,
    /// Move cursor up 1 row.
    UP          = 0x01,
    /// Move cursor down 1 row.
    DOWN        = 0x02,
    /// Move cursor right 1 column.
    RIGHT       = 0x03,
    /// Move cursor left 1 column.
    LEFT        = 0x04,
    HOME        = 0x05,
    END         = 0x06,
    INSERT      = 0x07,
    DELETE      = 0x08,
    PAGE_UP     = 0x09,
    PAGE_DOWN   = 0x0A,
    FUNCTION_1  = 0x0B,
    FUNCTION_2  = 0x0C,
    FUNCTION_3  = 0x0D,
    FUNCTION_4  = 0x0E,
    FUNCTION_5  = 0x0F,
    FUNCTION_6  = 0x10,
    FUNCTION_7  = 0x11,
    FUNCTION_8  = 0x12,
    FUNCTION_9  = 0x13,
    FUNCTION_10 = 0x14,
    FUNCTION_11 = 0x15,
    FUNCTION_12 = 0x16,
    ESCAPE      = 0x17,

    FUNCTION_13 = 0x68,
    FUNCTION_14 = 0x69,
    FUNCTION_15 = 0x6A,
    FUNCTION_16 = 0x6B,
    FUNCTION_17 = 0x6C,
    FUNCTION_18 = 0x6D,
    FUNCTION_19 = 0x6E,
    FUNCTION_20 = 0x6F,
    FUNCTION_21 = 0x70,
    FUNCTION_22 = 0x71,
    FUNCTION_23 = 0x72,
    FUNCTION_24 = 0x73,

    MUTE        = 0x7F,
    VOLUME_UP   = 0x80,
    VOLUME_DOWN = 0x81,

    BRIGHTNESS_UP   = 0x100,
    BRIGHTNESS_DOWN = 0x101,
    SUSPEND         = 0x102,
    HIBERNATE       = 0x103,
    TOGGLE_DISPLAY  = 0x104,
    RECOVERY        = 0x105,
    EJECT           = 0x106,
}}


