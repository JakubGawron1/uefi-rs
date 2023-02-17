//! Text I/O.

mod input_ex;
pub use self::input_ex::{Input_ex, Key_ex, ScanCode_ex};

mod input;
pub use self::input::{Input, Key, ScanCode};

mod output;
pub use self::output::{Color, Output, OutputMode};
