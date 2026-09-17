//! Shared enums. CLI parsing has been removed — the app is GUI-only.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang { En, Ru }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action { Update, Icons, Infinite, Validate }