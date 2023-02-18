use core::ffi::c_void;
use crate::proto::unsafe_protocol;
use crate::{Char16, Event, Result, Status};
use crate::proto::console::text::input::{Key, RawKey};
use core::mem::MaybeUninit;


#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct KeyState {
    pub key_shift_state: u32,
    pub key_toggle_state: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct KeyData {
    pub key: Key,
    pub key_state: KeyState,
}

#[repr(C)]
#[unsafe_protocol("dd9e7534-7762-4698-8c14-f58517a625aa")]
pub struct InputEx {
    reset: extern "efiapi" fn(this: &mut InputEx, extended: bool ) -> Status,
    read_key_stroke_ex: extern "efiapi" fn(this: &mut InputEx, key: *mut RawKey) -> Status,
    wait_for_key_ex: Event,
    set_state: extern "efiapi" fn(this: &mut InputEx, key_toggle_state: u8) -> Status,
    register_key_notify: extern "efiapi" fn(this: &mut InputEx, key_data: KeyData, key_notify: &mut KeyData, c_void),
    unregister_key_notify: extern "efiapi" fn(this: &mut InputEx, c_void),



}

impl InputEx {
    pub fn reset(&mut self, extended_verification: bool) -> Result {
        (self.reset)(self, extended_verification).into()
    }

    pub fn read_key_ex(&mut self) -> Result<Option<Key>> {
        let mut key = MaybeUninit::<RawKey>::uninit();

        match (self.read_key_stroke_ex)(self, key.as_mut_ptr()) {
            Status::NOT_READY => Ok(None),

            other => other.into_with_val(|| Some(unsafe { key.assume_init() }.into())),
        }
    }

    /// Event to be used with `BootServices::wait_for_event()` in order to wait
    /// for a key to be available
    #[must_use]
    pub const fn wait_for_key_event_ex(&self) -> &Event {
        &self.wait_for_key_ex
    }

}


newtype_enum! {
/// A keyboard scan code
///
/// Codes 0x8000 -> 0xFFFF are reserved for future OEM extensibility, therefore
/// this C enum is _not_ safe to model as a Rust enum (where the compiler must
/// know about all variants at compile time).
pub enum ScanCodeEx: u16 => #[allow(missing_docs)] {
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

    DIGIT1          = 0x1E,
}}


