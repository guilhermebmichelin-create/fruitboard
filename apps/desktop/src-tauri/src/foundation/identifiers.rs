use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct OpaqueId(String);

impl OpaqueId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn new(kind: IdKind, value: impl std::fmt::Display) -> Self {
        Self(format!("{}_{}", kind.prefix(), value))
    }

    #[cfg(test)]
    pub(crate) fn deterministic(kind: IdKind, value: u64) -> Self {
        Self::new(kind, format_args!("{value:032x}"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdKind {
    Correlation,
    Job,
}

impl IdKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Correlation => "correlation",
            Self::Job => "job",
        }
    }
}

pub trait IdGenerator: Send + Sync {
    fn next_id(&self, kind: IdKind) -> OpaqueId;
}

#[derive(Debug, Default)]
pub struct SystemIdGenerator;

impl IdGenerator for SystemIdGenerator {
    fn next_id(&self, kind: IdKind) -> OpaqueId {
        OpaqueId::new(kind, Uuid::now_v7().simple())
    }
}
