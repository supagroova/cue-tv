//! Stable public codes; never serialize third-party error chains or source URLs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureCode {
    InvalidMetadata,
    UnsupportedFormat,
    SourceTooLarge,
    LowStorage,
    Read,
}
impl FailureCode {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidMetadata => "invalid_metadata",
            Self::UnsupportedFormat => "unsupported_format",
            Self::SourceTooLarge => "source_too_large",
            Self::LowStorage => "low_storage",
            Self::Read => "read",
        }
    }
    pub fn snapshot(self) -> serde_json::Value {
        serde_json::json!({"state":"failed", "error_code":self.code(), "error":"Playback failed"})
    }
    pub fn from_error(error: &anyhow::Error) -> Self {
        error.downcast_ref::<Self>().copied().unwrap_or(Self::Read)
    }
}
impl std::fmt::Display for FailureCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for FailureCode {}
