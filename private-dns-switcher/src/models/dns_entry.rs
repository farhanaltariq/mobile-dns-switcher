use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsEntry {
    pub id: u64,
    pub name: String,
    pub hostname: String,
}

impl DnsEntry {
    #[must_use]
    pub fn new(id: u64, name: impl Into<String>, hostname: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            hostname: hostname.into(),
        }
    }
}
