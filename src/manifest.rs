//! Update-manifest parser and fail-closed validator.
//! Mirrors :SetManifestField and the checks inside :FixUpdate.

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manifest {
    pub version: Option<String>,
    pub size:    Option<u64>,
    pub sha256:  Option<String>,
    pub sig:     Option<String>,
    pub url:     Option<String>,
    pub file:    Option<String>,
}

impl Manifest {
    /// Parses `key=value` lines. Blank lines and `#` comments are skipped.
    /// Unknown keys are ignored, matching the batch behaviour.
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
                _ => {}
            }
        }
        Ok(m)
    }

    /// Fail-closed check, same order as the batch script.
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
    fn missing_sha256_fails() {
        let text = "\
            version=1.0.0\n\
            size=123\n\
            sig=test\n\
            url=/updates/update.gcup\n";

        let manifest = Manifest::parse(text).unwrap();

        assert!(manifest.validate().is_err());
    }

    #[test]
    fn invalid_sha256_fails() {
        let text = "\
            version=1.0.0\n\
            size=123\n\
            sha256=not-a-hash\n\
            sig=test\n\
            url=/updates/update.gcup\n";

        let manifest = Manifest::parse(text).unwrap();

        assert!(manifest.validate().is_err());
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let text = "\
            # comment\n\
            \n\
            size=123\n\
            url=/update.gcup\n\
            sha256=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
            sig=test\n";

        let manifest = Manifest::parse(text).unwrap();

        assert_eq!(manifest.size, Some(123));
        assert_eq!(manifest.url.as_deref(), Some("/update.gcup"));
    }
}