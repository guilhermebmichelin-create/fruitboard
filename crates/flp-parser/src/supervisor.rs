//! Native-only supervision of the trusted parser executable.
//!
//! This transport validates the protocol envelope. Successful result bodies
//! remain untrusted metadata: the application must validate field semantics
//! and current file/root revisions before using or persisting them.

use crate::ExpectedFingerprint;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const MAX_REQUEST_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const MAX_ALLOWED_ROOTS: usize = 256;
const CANCELLATION_POLL: Duration = Duration::from_millis(10);

#[derive(Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy)]
pub struct SupervisorLimits {
    pub request_timeout: Duration,
    pub max_requests_per_process: u32,
}

impl Default for SupervisorLimits {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(10),
            max_requests_per_process: 256,
        }
    }
}

/// Fixed native diagnostic categories. Neither paths nor subprocess text are
/// included in these errors, including spawn and pipe failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupervisorError {
    InvalidConfiguration,
    InvalidRequest,
    RequestLimit,
    StartFailed,
    ProcessStopped,
    TimedOut,
    Cancelled,
    ResponseLimit,
    InvalidProtocol,
    ShutdownFailed,
}

/// Result bodies and rejection codes are bounded but untrusted protocol data.
/// They deliberately have no Debug implementation that could log raw metadata.
pub enum ProtocolReply {
    Result(Value),
    Rejected { code: String },
}

pub struct ParseRequest {
    pub path: PathBuf,
    pub expected: ExpectedFingerprint,
    pub allowed_roots: Vec<PathBuf>,
}

pub enum ParserRequest {
    Describe,
    HealthCheck,
    Parse(ParseRequest),
}

struct RunningProcess {
    child: Child,
    requests: SyncSender<Vec<u8>>,
    replies: Receiver<Result<Vec<u8>, SupervisorError>>,
    worker: JoinHandle<()>,
    completed_requests: u32,
    reusable: bool,
}

/// One trusted native caller, one process, and one request at a time. The
/// mutable request API serializes access; it never retries a failed request.
/// The next request may start a new process after successful retirement.
pub struct ParserSupervisor {
    executable: PathBuf,
    limits: SupervisorLimits,
    running: Option<RunningProcess>,
    sequence: u64,
    #[cfg(test)]
    arguments: Vec<std::ffi::OsString>,
}

impl ParserSupervisor {
    /// Native lifecycle observation only. This is the currently owned child,
    /// including an exited child awaiting retirement, not proof of liveness.
    /// Never use a PID alone as authority to terminate a process.
    pub fn process_id(&self) -> Option<u32> {
        self.running.as_ref().map(|running| running.child.id())
    }

    pub fn new(executable: PathBuf, limits: SupervisorLimits) -> Result<Self, SupervisorError> {
        if !absolute_without_parent(&executable)
            || limits.request_timeout.is_zero()
            || limits.request_timeout > Duration::from_secs(60)
            || limits.max_requests_per_process == 0
            || limits.max_requests_per_process > 10_000
        {
            return Err(SupervisorError::InvalidConfiguration);
        }
        Ok(Self {
            executable,
            limits,
            running: None,
            sequence: 0,
            #[cfg(test)]
            arguments: Vec::new(),
        })
    }

    pub fn request(
        &mut self,
        request: ParserRequest,
        cancellation: &CancellationToken,
    ) -> Result<ProtocolReply, SupervisorError> {
        if cancellation.is_cancelled() {
            return Err(SupervisorError::Cancelled);
        }
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(SupervisorError::InvalidRequest)?;
        // Unique for this supervisor lifetime, including across restarts.
        let id = format!(
            "00000000-0000-0000-{:04x}-{:012x}",
            self.sequence >> 48,
            self.sequence & 0xffff_ffff_ffff
        );
        let bytes = encode_request(request, &id)?;
        self.ensure_running()?;
        let running = self.running.as_mut().expect("started");
        let deadline = Instant::now() + self.limits.request_timeout;
        let reply = if running.requests.send(bytes).is_err() {
            Err(SupervisorError::ProcessStopped)
        } else {
            loop {
                if cancellation.is_cancelled() {
                    break Err(SupervisorError::Cancelled);
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break Err(SupervisorError::TimedOut);
                }
                match running
                    .replies
                    .recv_timeout(remaining.min(CANCELLATION_POLL))
                {
                    Ok(Ok(bytes)) => {
                        let decoded = decode_reply(&bytes, &id);
                        if cancellation.is_cancelled() {
                            break Err(SupervisorError::Cancelled);
                        }
                        if Instant::now() >= deadline {
                            break Err(SupervisorError::TimedOut);
                        }
                        break decoded;
                    }
                    Ok(Err(error)) => break Err(error),
                    Err(RecvTimeoutError::Disconnected) => {
                        break Err(SupervisorError::ProcessStopped);
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                }
            }
        };
        if reply.is_err() {
            // A delayed or additional reply must never satisfy a later call.
            running.reusable = false;
            self.shutdown()?;
        } else {
            running.completed_requests += 1;
        }
        reply
    }

    fn ensure_running(&mut self) -> Result<(), SupervisorError> {
        let retire = if let Some(running) = &mut self.running {
            !running.reusable
                || running.completed_requests >= self.limits.max_requests_per_process
                || match running.child.try_wait() {
                    Ok(status) => status.is_some(),
                    Err(_) => {
                        running.reusable = false;
                        true
                    }
                }
        } else {
            false
        };
        if retire {
            self.shutdown()?;
        }
        if self.running.is_some() {
            return Ok(());
        }
        let mut command = Command::new(&self.executable);
        command.stdin(Stdio::piped()).stdout(Stdio::piped());
        // Arbitrary diagnostics can contain input paths. Discard them at the
        // OS handle rather than buffering or forwarding them into app logs.
        command.stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        #[cfg(test)]
        command.args(&self.arguments);
        let mut child = command.spawn().map_err(|_| SupervisorError::StartFailed)?;
        let mut stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let (requests, incoming) = mpsc::sync_channel::<Vec<u8>>(1);
        let (outgoing, replies) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("fruitboard-parser-io".into())
            .spawn(move || {
                let mut reader = BufReader::new(stdout);
                while let Ok(bytes) = incoming.recv() {
                    let response = stdin
                        .write_all(&bytes)
                        .and_then(|()| stdin.flush())
                        .map_err(|_| SupervisorError::ProcessStopped)
                        .and_then(|()| read_response(&mut reader));
                    let failed = response.is_err();
                    if outgoing.send(response).is_err() || failed {
                        break;
                    }
                }
            });
        let worker = match worker {
            Ok(worker) => worker,
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(SupervisorError::StartFailed);
            }
        };
        self.running = Some(RunningProcess {
            child,
            requests,
            replies,
            worker,
            completed_requests: 0,
            reusable: true,
        });
        Ok(())
    }

    /// Terminate and reap the owned process and release its pipe worker.
    /// Failure retains the non-reusable handle, so no replacement is started
    /// while the prior process cannot be confirmed stopped.
    pub fn shutdown(&mut self) -> Result<(), SupervisorError> {
        let Some(running) = &mut self.running else {
            return Ok(());
        };
        running.reusable = false;
        running
            .child
            .kill()
            .map_err(|_| SupervisorError::ShutdownFailed)?;
        running
            .child
            .wait()
            .map_err(|_| SupervisorError::ShutdownFailed)?;
        let RunningProcess {
            requests,
            replies,
            worker,
            ..
        } = self.running.take().expect("owned process");
        drop(requests);
        drop(replies);
        worker.join().map_err(|_| SupervisorError::ShutdownFailed)
    }
}

impl Drop for ParserSupervisor {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn absolute_without_parent(path: &Path) -> bool {
    path.is_absolute() && !path.components().any(|part| part == Component::ParentDir)
}

fn encode_request(request: ParserRequest, id: &str) -> Result<Vec<u8>, SupervisorError> {
    let mut value = json!({"protocolVersion":crate::PROTOCOL_VERSION,"schemaVersion":crate::SCHEMA_VERSION,"id":id});
    match request {
        ParserRequest::Describe => value["method"] = json!("describe"),
        ParserRequest::HealthCheck => value["method"] = json!("healthCheck"),
        ParserRequest::Parse(request) => {
            if request.allowed_roots.len() > MAX_ALLOWED_ROOTS {
                return Err(SupervisorError::RequestLimit);
            }
            if !absolute_without_parent(&request.path)
                || request
                    .allowed_roots
                    .iter()
                    .any(|root| !absolute_without_parent(root))
            {
                return Err(SupervisorError::InvalidRequest);
            }
            let path = request
                .path
                .to_str()
                .ok_or(SupervisorError::InvalidRequest)?;
            let roots = request
                .allowed_roots
                .iter()
                .map(|root| root.to_str().ok_or(SupervisorError::InvalidRequest))
                .collect::<Result<Vec<_>, _>>()?;
            // Bound caller-controlled input before JSON allocates escaped
            // copies. The final serialized line limit remains authoritative.
            let text_bytes = roots
                .iter()
                .try_fold(path.len(), |total, root| total.checked_add(root.len()));
            if text_bytes.is_none_or(|total| total > MAX_REQUEST_BYTES) {
                return Err(SupervisorError::RequestLimit);
            }
            value["method"] = json!("parse");
            value["params"] = json!({
                "path":path,
                "expected":{"size":request.expected.size,"modifiedAtMs":request.expected.modified_at_ms},
                "allowedRoots":roots,"features":["basic-metadata"]
            });
        }
    }
    let mut bytes = serde_json::to_vec(&value).map_err(|_| SupervisorError::InvalidRequest)?;
    if bytes.len() >= MAX_REQUEST_BYTES {
        return Err(SupervisorError::RequestLimit);
    }
    bytes.push(b'\n');
    Ok(bytes)
}

fn read_response(reader: &mut impl BufRead) -> Result<Vec<u8>, SupervisorError> {
    let mut response = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .map_err(|_| SupervisorError::ProcessStopped)?;
        if available.is_empty() {
            return Err(SupervisorError::ProcessStopped);
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(available.len(), |index| index + 1);
        if response.len().saturating_add(count) > MAX_RESPONSE_BYTES + 1 {
            return Err(SupervisorError::ResponseLimit);
        }
        response.extend_from_slice(&available[..count]);
        reader.consume(count);
        if newline.is_some() {
            response.pop();
            return Ok(response);
        }
    }
}

fn decode_reply(bytes: &[u8], id: &str) -> Result<ProtocolReply, SupervisorError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| SupervisorError::InvalidProtocol)?;
    let object = value.as_object().ok_or(SupervisorError::InvalidProtocol)?;
    if value["protocolVersion"].as_u64() != Some(crate::PROTOCOL_VERSION)
        || value["schemaVersion"].as_u64() != Some(crate::SCHEMA_VERSION)
        || value["id"].as_str() != Some(id)
        || object.contains_key("result") == object.contains_key("error")
    {
        return Err(SupervisorError::InvalidProtocol);
    }
    if let Some(result) = object.get("result") {
        if !result.is_object() {
            return Err(SupervisorError::InvalidProtocol);
        }
        Ok(ProtocolReply::Result(result.clone()))
    } else {
        let code = value["error"]["code"]
            .as_str()
            .ok_or(SupervisorError::InvalidProtocol)?;
        if code.is_empty()
            || code.len() > 64
            || !code
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(SupervisorError::InvalidProtocol);
        }
        Ok(ProtocolReply::Rejected {
            code: code.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests;
