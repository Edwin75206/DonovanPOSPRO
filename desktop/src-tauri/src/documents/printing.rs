//! Platform adapters use the same receipt layout and never perform a checkout.
#[cfg(windows)]
#[path = "printing_windows.rs"]
mod platform;
#[cfg(not(windows))]
#[path = "printing_cups.rs"]
mod platform;
pub use platform::*;
// Compile the CUPS adapter on Windows as well; OS integration still needs a Mac.
#[cfg(all(test, windows))]
#[allow(dead_code)]
#[path = "printing_cups.rs"]
mod cups_compile_check;
