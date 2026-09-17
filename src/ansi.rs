//! ANSI color codes used by the fixers' log output.
//!
//! The GUI parses these escapes and renders the log with matching colors.
//! There is no VT-enable or console pause anymore: this build is GUI-only.

pub const RED:    &str = "\x1b[91m";
pub const GREEN:  &str = "\x1b[92m";
pub const YELLOW: &str = "\x1b[93m";
pub const BLUE:   &str = "\x1b[94m";
pub const CYAN:   &str = "\x1b[96m";
pub const GRAY:   &str = "\x1b[90m";
pub const BOLD:   &str = "\x1b[1m";
pub const RESET:  &str = "\x1b[0m";