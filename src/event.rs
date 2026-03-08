use compact_str::CompactString;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Event {
    pub event: CompactString,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<CompactString>,
}
