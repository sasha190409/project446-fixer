//! Update-manifest parser and fail-closed validator.

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manifest {
    pub version: Option<String>,
    pub size:    Option<u64>,
    pub sha256:  Option<String>,
    pub sig:     Option<String>,
    pub url:     Option<String>,
    pub file:    Option<String>,
    /// Пункт 14: минимальная версия фиксера, требуемая для установки
    /// этого обновления. `None` = совместимо с любой версией.
    pub fixer_min_version: Option<String>,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self> {
        let mut m = Manifest::default();
        for (i, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (k, v) = line
                .split_once('=')
                .with_context(|| format!("manifest line {}: no '=' in {:?}", i + 1, raw))?;
            let k = k.trim().to_ascii_lowercase();
            let v = v.trim();
            match k.as_str() {
                "version" => m.version = Some(v.to_string()),
                "size" => {
                    m.size = Some(
                        v.parse::<u64>()
                            .with_context(|| format!("size is not u64: {:?}", v))?,
                    )
                }
                "sha256" => m.sha256 = Some(v.to_ascii_lowercase()),
                "sig"    => m.sig    = Some(v.to_string()),
                "url"    => m.url    = Some(v.to_string()),
                "file"   => m.file   = Some(v.to_string()),
                "fixer_min_version" | "fixer-min-version" =>
                    m.fixer_min_version = Some(v.to_string()),
                _ => {}
            }
        }
        Ok(m)
    }

    pub fn validate(&self) -> Result<()> {
        if self.url.is_none() {
            bail!("manifest: missing 'url'");
        }
        if self.size.is_none() {
            bail!("manifest: missing 'size' (fail-closed)");
        }
        if self.sha256.is_none() {
            bail!("manifest: missing 'sha256' (fail-closed)");
        }
        let sha256 = self.sha256.as_deref().unwrap();
        if sha256.len() != 64 || !sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            bail!("manifest: invalid 'sha256'");
        }
        if self.sig.is_none() {
            bail!("manifest: missing 'sig' (fail-closed)");
        }
        Ok(())
    }

    /// Пункт 14: сравнение `fixer_min_version` с текущей версией бинаря.
    /// Возвращает `Ok(())`, если совместимо (в т.ч. когда поля нет), и
    /// `Err` с человекочитаемым текстом — иначе.
    pub fn check_fixer_version(&self) -> Result<()> {
        let Some(min) = self.fixer_min_version.as_deref() else {
            return Ok(());
        };
        let current = env!("CARGO_PKG_VERSION");
        if version_ge(current, min) {
            Ok(())
        } else {
            bail!(
                "manifest requires fixer version >= {}, this build is {}",
                min, current
            )
        }
    }
}

/// Сравнение semver-подобных строк без внешнего крейта.
/// Возвращает `true`, если `a >= b`. Не падает при мусоре: несовпадение
/// формата → консервативно `false` (устаревшая версия).
fn version_ge(a: &str, b: &str) -> bool {
    fn parts(s: &str) -> Option<Vec<u32>> {
        s.split('-').next()?
            .split('.')
            .map(|x| x.parse::<u32>().ok())
            .collect()
    }
    let (Some(av), Some(bv)) = (parts(a), parts(b)) else { return false };
    for i in 0..av.len().max(bv.len()) {
        let ai = av.get(i).copied().unwrap_or(0);
        let bi = bv.get(i).copied().unwrap_or(0);
        if ai != bi {
            return ai > bi;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_manifest() {
        let text = "\
            version=1.0.0\n\
            size=123\n\
            sha256=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
            sig=test\n\
            url=/updates/update.gcup\n\
            file=update.gcup\n";
        let manifest = Manifest::parse(text).unwrap();
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn min_version_absent_is_ok() {
        let m = Manifest::default();
        assert!(m.check_fixer_version().is_ok());
    }

    #[test]
    fn min_version_older_ok() {
        let mut m = Manifest::default();
        m.fixer_min_version = Some("1.0.0".into());
        assert!(m.check_fixer_version().is_ok());
    }

    #[test]
    fn min_version_newer_fails() {
        let mut m = Manifest::default();
        m.fixer_min_version = Some("999.0.0".into());
        assert!(m.check_fixer_version().is_err());
    }
}
