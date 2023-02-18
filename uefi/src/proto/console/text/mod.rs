//! Text I/O.

mod input_ex;
pub use self::input_ex::{InputEx, ScanCodeEx};

mod input;
pub use self::input::{Input, Key, ScanCode};

mod output;
pub use self::output::{Color, Output, OutputMode};
