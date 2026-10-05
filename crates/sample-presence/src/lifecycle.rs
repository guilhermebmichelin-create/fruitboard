use crate::{DEADLINE_MS, RequestFailure};
use std::sync::{Arc, Mutex};

/// Host-owned monotonic milliseconds. Wall time never drives deadlines.
pub trait MonotonicClock: Send + Sync {
    fn now_ms(&self) -> u64;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionError {
    Busy,
    Closed,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum State {
    Active,
    Cancelled,
    Deadline,
    Stale,
    Published,
    Retired,
}

struct RequestState {
    state: Mutex<State>,
    started: u64,
    clock: Arc<dyn MonotonicClock>,
}

/// May be held by a timer while the worker is blocked. It never releases the gate.
#[derive(Clone)]
pub struct RequestControl(Arc<RequestState>);

impl RequestControl {
    fn stop(&self, state: State) {
        let mut current = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        if *current == State::Active {
            *current = state;
        }
    }
    pub fn cancel(&self) {
        self.stop(State::Cancelled);
    }
    pub fn invalidate(&self) {
        self.stop(State::Stale);
    }
    /// Fences timeout publication; no promise of interrupting a kernel syscall.
    pub fn poll_deadline(&self) -> Result<(), RequestFailure> {
        self.checkpoint()
    }
    pub(crate) fn checkpoint(&self) -> Result<(), RequestFailure> {
        let mut state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        self.check_locked(&mut state)
    }
    fn check_locked(&self, state: &mut State) -> Result<(), RequestFailure> {
        if *state == State::Active {
            let now = self.0.clock.now_ms();
            // A broken/backward monotonic source also fails closed.
            if now < self.0.started || now - self.0.started >= DEADLINE_MS {
                *state = State::Deadline;
            }
        }
        match *state {
            State::Active => Ok(()),
            State::Cancelled => Err(RequestFailure::Cancelled),
            State::Deadline => Err(RequestFailure::Deadline),
            State::Stale => Err(RequestFailure::Stale),
            State::Published | State::Retired => Err(RequestFailure::WorkerRetired),
        }
    }
    pub(crate) fn publish(&self) -> Result<(), RequestFailure> {
        let mut state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        self.check_locked(&mut state)?;
        *state = State::Published;
        Ok(())
    }
}

#[derive(Default)]
struct GateState {
    closed: bool,
    active: Option<RequestControl>,
}

/// One instance in the eventual native host, shared by all requests. No queue.
#[derive(Clone, Default)]
pub struct WorkerGate(Arc<Mutex<GateState>>);

impl WorkerGate {
    pub fn try_start(&self, clock: Arc<dyn MonotonicClock>) -> Result<WorkerLease, AdmissionError> {
        let mut gate = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if gate.closed {
            return Err(AdmissionError::Closed);
        }
        if gate.active.is_some() {
            return Err(AdmissionError::Busy);
        }
        let control = RequestControl(Arc::new(RequestState {
            state: Mutex::new(State::Active),
            started: clock.now_ms(),
            clock,
        }));
        gate.active = Some(control.clone());
        Ok(WorkerLease {
            gate: self.clone(),
            control,
            started: false,
        })
    }
    pub fn shutdown(&self) {
        let mut gate = self.0.lock().unwrap_or_else(|e| e.into_inner());
        gate.closed = true;
        if let Some(active) = &gate.active {
            active.invalidate();
        }
    }
}

/// Move into the sole worker; retain until every underlying operation retires.
/// Drop releases admission. Cancellation only fences publication.
pub struct WorkerLease {
    gate: WorkerGate,
    control: RequestControl,
    started: bool,
}

impl WorkerLease {
    pub fn control(&self) -> RequestControl {
        self.control.clone()
    }
    pub(crate) fn begin(&mut self) -> Result<(), RequestFailure> {
        if self.started {
            return Err(RequestFailure::WorkerRetired);
        }
        self.started = true;
        self.control.checkpoint()
    }
    pub(crate) fn publish(&self) -> Result<(), RequestFailure> {
        self.control.publish()
    }
}

impl Drop for WorkerLease {
    fn drop(&mut self) {
        let mut gate = self.gate.0.lock().unwrap_or_else(|e| e.into_inner());
        if gate
            .active
            .as_ref()
            .is_some_and(|c| Arc::ptr_eq(&c.0, &self.control.0))
        {
            self.control.stop(State::Retired);
            gate.active = None;
        }
    }
}
