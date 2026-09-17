//! CS:GO Legacy Fixer — GUI-only Rust port.

// ---- UI layer must come FIRST: `ui` uses std's real println! ----
pub mod ui;

// ---- Shadow `println!` for everything declared below ----
//
// The fixers keep writing `println!(...)`; those calls are routed through
// `ui::say_line` into the egui log panel via a channel.

macro_rules! println {
    () => { $crate::ui::say_line_str("") };
    ($($t:tt)+) => { $crate::ui::say_line(format_args!($($t)+)) };
}

/// Bail out of a `Result<()>` function if the user requested cancellation.
/// Prints a status line and opens the Continue modal.
macro_rules! check_cancel {
    ($msgs:expr) => {
        if $crate::ui::is_cancelled() {
            $crate::ui::say_line(format_args!(
                "{}{}{}",
                $crate::ansi::YELLOW,
                $msgs.cancelled,
                $crate::ansi::RESET
            ));
            $crate::ui::pause();
            return Ok(());
        }
    };
}

pub mod ansi;
pub mod args;
pub mod crypto;
pub mod i18n;
pub mod manifest;
pub mod url;
pub mod vdf;

#[cfg(windows)]
pub mod fixer;
#[cfg(windows)]
pub mod gui;
#[cfg(windows)]
pub mod paths;
#[cfg(windows)]
pub mod win;