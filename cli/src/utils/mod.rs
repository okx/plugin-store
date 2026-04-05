pub mod cache;
pub mod platform;
pub mod ui;

/// Parse a SHA256 checksums file (sha256sum format) and return the hash for `asset_name`.
pub fn find_checksum(checksums_content: &str, asset_name: &str) -> Option<String> {
    for line in checksums_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 && parts[1].trim_start_matches('*') == asset_name {
            return Some(parts[0].to_string());
        }
    }
    None
}
