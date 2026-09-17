//! Full-URL resolution. Exact port of :BuildFullUrl from the batch.

pub fn build_full_url(mirror: &str, manifest_url: &str) -> String {
    if manifest_url.starts_with("http://") || manifest_url.starts_with("https://") {
        return manifest_url.to_string();
    }

    let base = mirror_base(mirror);
    let rel = manifest_url.strip_prefix('/').unwrap_or(manifest_url);
    format!("{}/{}", base, rel)
}

/// Strip the trailing manifest path from a mirror URL.
/// `https://host/api/update/manifest?platform=win32` → `https://host`
pub fn mirror_base(mirror: &str) -> String {
    let mut base = mirror.split('?').next().unwrap_or("").to_string();

    for suffix in ["/api/update/manifest", "manifest.txt"] {
        if base.ends_with(suffix) {
            base.truncate(base.len() - suffix.len());
        }
    }
    if base.ends_with('/') {
        base.pop();
    }
    if base.ends_with("%2F") {
        base.truncate(base.len() - 3);
    }
    base
}