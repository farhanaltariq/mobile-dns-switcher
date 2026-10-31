use std::fs;
use std::path::PathBuf;

use crate::models::DnsEntry;

const FILE_NAME: &str = "dns_entries.json";

#[must_use]
pub fn default_entries() -> Vec<DnsEntry> {
    vec![
        DnsEntry::new(1, "Quad9", "dns.quad9.net"),
        DnsEntry::new(2, "Google", "dns.google"),
        DnsEntry::new(3, "Cloudflare", "1dot1dot1dot1.cloudflare-dns.com"),
    ]
}

#[must_use]
pub fn storage_path() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        PathBuf::from("/data/data/com.sicloud.dnsswitcher/files").join(FILE_NAME)
    }
    #[cfg(not(target_os = "android"))]
    {
        std::env::temp_dir().join(FILE_NAME)
    }
}

#[must_use]
pub fn load() -> Vec<DnsEntry> {
    let path = storage_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|json| serde_json::from_str::<Vec<DnsEntry>>(&json).ok())
        .filter(|list| !list.is_empty())
        .unwrap_or_else(default_entries)
}

pub fn save(entries: &[DnsEntry]) -> Result<(), String> {
    let path = storage_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

#[must_use]
pub fn next_id(entries: &[DnsEntry]) -> u64 {
    entries.iter().map(|e| e.id).max().unwrap_or(0) + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_and_next_id() {
        let d = default_entries();
        assert_eq!(d.len(), 3);
        assert_eq!(next_id(&d), 4);
        assert_eq!(next_id(&[]), 1);
    }
}
