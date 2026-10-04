//! Shared enums. CLI parsing has been removed — the app is GUI-only.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang { En, Ru }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action { Update, Icons, Infinite, Validate }

impl Lang {
    /// Maps a Win32 `LANGID` to our supported language set.
    ///
    /// Russian, Ukrainian and Belarusian all collapse to `Lang::Ru`
    /// (per product requirement). Everything else → `Lang::En`.
    ///
    /// Only the *primary* language id (low byte) is considered, so sublanguages
    /// like `0x0819` (Russian – Moldova) still resolve to `Ru`.
    pub fn from_langid(langid: u16) -> Self {
        // Windows LANG_* constants (winnt.h):
        //   LANG_RUSSIAN    = 0x19
        //   LANG_UKRAINIAN  = 0x22
        //   LANG_BELARUSIAN = 0x23
        match (langid & 0xFF) as u8 {
            0x19 | 0x22 | 0x23 => Lang::Ru,
            _ => Lang::En,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Lang;

    #[test]
    fn russian_primary_langid() {
        assert_eq!(Lang::from_langid(0x0419), Lang::Ru);
    }

    #[test]
    fn ukrainian_primary_langid() {
        assert_eq!(Lang::from_langid(0x0422), Lang::Ru);
    }

    #[test]
    fn belarusian_primary_langid() {
        assert_eq!(Lang::from_langid(0x0423), Lang::Ru);
    }

    #[test]
    fn russian_moldova_sublang_still_russian() {
        // 0x0819 = Russian (Moldova) — same primary id (0x19).
        assert_eq!(Lang::from_langid(0x0819), Lang::Ru);
    }

    #[test]
    fn english_primary_langid() {
        assert_eq!(Lang::from_langid(0x0409), Lang::En);
    }

    #[test]
    fn other_langs_are_english() {
        assert_eq!(Lang::from_langid(0x0407), Lang::En); // German
        assert_eq!(Lang::from_langid(0x040C), Lang::En); // French
        assert_eq!(Lang::from_langid(0x0411), Lang::En); // Japanese
        assert_eq!(Lang::from_langid(0x0000), Lang::En); // LANG_NEUTRAL
    }
}
