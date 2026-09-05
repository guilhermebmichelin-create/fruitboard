use super::clock::Clock;
use super::identifiers::{IdGenerator, IdKind, OpaqueId};
use super::logging::{LogSink, OperationalLogEvent};
use std::fmt::{Display, Formatter};
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

#[derive(Debug)]
pub(crate) struct FakeClock {
    now: AtomicU64,
}

impl FakeClock {
    pub(crate) fn new(now: u64) -> Self {
        Self {
            now: AtomicU64::new(now),
        }
    }

    pub(crate) fn set(&self, now: u64) {
        self.now.store(now, Ordering::SeqCst);
    }
}

impl Clock for FakeClock {
    fn now_millis(&self) -> u64 {
        self.now.load(Ordering::SeqCst)
    }
}

#[derive(Debug)]
pub(crate) struct FakeIdGenerator {
    next: AtomicU64,
}

impl FakeIdGenerator {
    pub(crate) fn new(first: u64) -> Self {
        Self {
            next: AtomicU64::new(first),
        }
    }
}

impl IdGenerator for FakeIdGenerator {
    fn next_id(&self, kind: IdKind) -> OpaqueId {
        OpaqueId::deterministic(kind, self.next.fetch_add(1, Ordering::SeqCst))
    }
}

#[derive(Debug, Default)]
pub(crate) struct RecordingLogSink {
    events: Mutex<Vec<OperationalLogEvent>>,
}

impl RecordingLogSink {
    pub(crate) fn events(&self) -> Vec<OperationalLogEvent> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl LogSink for RecordingLogSink {
    fn write(&self, event: &OperationalLogEvent) -> io::Result<()> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event.clone());
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct FakeError {
    message: &'static str,
}

impl FakeError {
    pub(crate) fn new(message: &'static str) -> Self {
        Self { message }
    }
}

impl Display for FakeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for FakeError {}
