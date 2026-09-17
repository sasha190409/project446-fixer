//! UI abstraction: log forwarding + modal prompts.
//!
//! All output from the fixers goes through `say_line` / `say_inline`, which
//! forward to the egui thread over a channel. `pause()` blocks the worker
//! on a condvar until the GUI thread calls `resume()`.

use std::fmt::Arguments;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Condvar, Mutex, OnceLock};

/// Messages sent from worker → GUI thread.
#[derive(Debug)]
pub enum LogEvent {
    /// One line of output (may include ANSI escapes).
    Line(String),
    /// Worker is waiting for the user to acknowledge.
    Pause,
    /// Download progress. `total == 0` means "hide the bar".
    Progress { done: u64, total: u64 },
    /// Worker is done; GUI should re-enable buttons.
    Done,
}

struct Gui {
    tx: Sender<LogEvent>,
}

static GUI: OnceLock<Mutex<Option<Gui>>> = OnceLock::new();

fn gui() -> &'static Mutex<Option<Gui>> {
    GUI.get_or_init(|| Mutex::new(None))
}

/// Install the GUI sink. Called once by the eframe app before spawning a worker.
pub fn set_gui(tx: Sender<LogEvent>) {
    *gui().lock().unwrap_or_else(|e| e.into_inner()) = Some(Gui { tx });
}

/// Remove the GUI sink. Called when the worker has finished.
pub fn unset_gui() {
    *gui().lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn send(ev: LogEvent) {
    if let Some(g) = gui().lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = g.tx.send(ev);
    }
}

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------

/// Used by the shadowed `println!` macro.
pub fn say_line(args: Arguments) {
    send(LogEvent::Line(args.to_string()));
}

/// Used by the shadowed `println!` for the no-arg case.
pub fn say_line_str(s: &str) {
    send(LogEvent::Line(s.to_string()));
}

/// Used by the shadowed `print!` macro. In GUI mode this is equivalent to
/// `say_line`: the log widget is line-oriented.
pub fn say_inline(args: Arguments) {
    send(LogEvent::Line(args.to_string()));
}

/// Signal to the GUI that the worker finished.
pub fn finish() {
    send(LogEvent::Done);
}

// ---------------------------------------------------------------------------
// Progress
// ---------------------------------------------------------------------------

/// Report download progress. `total == 0` clears the bar.
pub fn progress(done: u64, total: u64) {
    send(LogEvent::Progress { done, total });
}

// ---------------------------------------------------------------------------
// Cancellation
// ---------------------------------------------------------------------------

static CANCELLED: AtomicBool = AtomicBool::new(false);

/// Clear the cancel flag. Called by the GUI right before spawning a worker.
pub fn reset_cancel() {
    CANCELLED.store(false, Ordering::SeqCst);
}

/// Ask the running worker to stop. Called by the GUI's Cancel button.
/// Returns `true` if this was the first request (state went from
/// "running" to "cancelling").
pub fn request_cancel() -> bool {
    !CANCELLED.swap(true, Ordering::SeqCst)
}

/// Has cancellation been requested? Checked by the worker.
pub fn is_cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}

/// `Err` if the user requested cancellation. Used by `?` inside functions
/// that return `anyhow::Result`.
pub fn ensure_not_cancelled() -> anyhow::Result<()> {
    if is_cancelled() {
        anyhow::bail!("cancelled by user")
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Pause gate
// ---------------------------------------------------------------------------

struct Gate {
    waiting: Mutex<bool>,
    cv: Condvar,
}

static GATE: OnceLock<Gate> = OnceLock::new();

fn gate() -> &'static Gate {
    GATE.get_or_init(|| Gate {
        waiting: Mutex::new(false),
        cv: Condvar::new(),
    })
}

/// Worker: ask the GUI to display a "Continue" prompt and block until the
/// user clicks it.
pub fn pause() {
    let g = gate();
    *g.waiting.lock().unwrap() = true;
    send(LogEvent::Pause);
    let mut flag = g.waiting.lock().unwrap();
    while *flag {
        flag = g.cv.wait(flag).unwrap();
    }
}

/// GUI: release the worker blocked inside `pause()`.
pub fn resume() {
    let g = gate();
    *g.waiting.lock().unwrap() = false;
    g.cv.notify_all();
}

// ---------------------------------------------------------------------------
// Modal prompts
// ---------------------------------------------------------------------------

/// Ask the user to pick a folder using the native dialog.
pub fn pick_folder(title: &str) -> Option<String> {
    rfd::FileDialog::new()
        .set_title(title)
        .pick_folder()
        .map(|p| p.to_string_lossy().into_owned())
}