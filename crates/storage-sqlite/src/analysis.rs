//! Native operational queue. No SQL, paths or source capability reaches IPC.
use crate::metadata;
use crate::{Database, MetadataInput, MetadataSnapshotHeader, Result, StorageError};
use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::ParserCapabilities;
use fruitboard_flp_parser::{ADAPTER_VERSION, MAX_FILE_BYTES, SCHEMA_VERSION};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::{Path, PathBuf};

pub const MAX_ANALYSIS_BATCH: usize = 128;
const MAX_ATTEMPTS: i64 = 3;
const LEASE_MS: i64 = 60_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisState {
    Queued,
    Running,
    Complete,
    Unsupported,
    Failed,
    Cancelled,
    Stale,
}
impl AnalysisState {
    fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "complete" => Self::Complete,
            "unsupported" => Self::Unsupported,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            "stale" => Self::Stale,
            _ => return Err(StorageError::InvalidSchema),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisFailure {
    Interrupted,
    SourceUnavailable,
    SourceChanged,
    ParserTransport,
    InvalidReply,
    Cancelled,
}
impl AnalysisFailure {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Interrupted => "INTERRUPTED",
            Self::SourceUnavailable => "SOURCE_UNAVAILABLE",
            Self::SourceChanged => "SOURCE_CHANGED",
            Self::ParserTransport => "PARSER_TRANSPORT",
            Self::InvalidReply => "INVALID_REPLY",
            Self::Cancelled => "CANCELLED",
        }
    }
}
pub struct AnalysisStatus {
    pub job_id: String,
    pub location_id: String,
    pub state: AnalysisState,
    pub attempt: i64,
    pub snapshot_id: Option<String>,
    pub error_code: Option<String>,
}
#[derive(Clone)]
pub struct AnalysisCursor(String);

/// Native-only file authority selected by storage, never caller-supplied paths.
pub struct AnalysisSource {
    root: PathBuf,
    path: PathBuf,
    byte_size: u64,
    modified_at_ns: i64,
    identity: Option<(u64, u128)>,
}
impl AnalysisSource {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn byte_size(&self) -> u64 {
        self.byte_size
    }
    pub fn modified_at_ns(&self) -> i64 {
        self.modified_at_ns
    }
    pub fn identity(&self) -> Option<(u64, u128)> {
        self.identity
    }
}
pub struct AnalysisLease {
    job_id: String,
    session_id: String,
    token: String,
    input: MetadataInput,
    source: AnalysisSource,
}
impl AnalysisLease {
    pub fn job_id(&self) -> &str {
        &self.job_id
    }
    pub fn location_id(&self) -> &str {
        &self.input.location_id
    }
    pub fn source(&self) -> &AnalysisSource {
        &self.source
    }
}

struct Cell {
    job_id: String,
    input: MetadataInput,
    state: AnalysisState,
    attempt: i64,
    version: String,
    schema: i64,
    due: i64,
}
fn cell(connection: &Connection, location: &str) -> Result<Option<Cell>> {
    connection.query_row("SELECT job_id,root_id,location_id,project_file_id,root_revision,file_revision,location_revision,
        publication_revision,byte_size,modified_at_ns,state,attempt,adapter_version,parser_schema_version,due_at_ms
        FROM analysis_job WHERE location_id=?1", [location], |row| {
        let size:i64=row.get(8)?;
        Ok(Cell {job_id:row.get(0)?,input:MetadataInput {
            root_id:row.get(1)?,location_id:row.get(2)?,project_file_id:row.get(3)?,root_revision:row.get(4)?,
            file_revision:row.get(5)?,location_revision:row.get(6)?,publication_revision:row.get(7)?,
            byte_size:u64::try_from(size).map_err(|_|rusqlite::Error::IntegralValueOutOfRange(8,size))?,modified_at_ns:row.get(9)?,
        },state:AnalysisState::parse(&row.get::<_,String>(10)?).map_err(|_|rusqlite::Error::InvalidQuery)?,
            attempt:row.get(11)?,version:row.get(12)?,schema:row.get(13)?,due:row.get(14)?})
    }).optional().map_err(Into::into)
}
fn clear(
    connection: &Connection,
    job_id: &str,
    state: &str,
    error: Option<&str>,
    due: i64,
) -> Result<()> {
    connection.execute(
        "UPDATE analysis_job SET state=?2,error_code=?3,due_at_ms=?4,
        session_id=NULL,lease_token=NULL,lease_until_ms=NULL WHERE job_id=?1",
        params![job_id, state, error, due],
    )?;
    Ok(())
}
fn matches_source(connection: &Connection, input: &MetadataInput) -> Result<bool> {
    match metadata::capture(connection, &input.root_id, &input.location_id) {
        Ok(current) => Ok(current == *input),
        Err(StorageError::NotFound) => Ok(false),
        Err(error) => Err(error),
    }
}
fn same_desired_source(left: &MetadataInput, right: &MetadataInput) -> bool {
    // Publication order is a commit fence, not a new source observation.
    // Another alias's negative result must not revive unchanged terminal work.
    let mut left = left.clone();
    left.publication_revision = right.publication_revision;
    left == *right
}
fn session_current(connection: &Connection, session: &str) -> Result<bool> {
    connection
        .query_row(
            "SELECT analysis_session_id IS ?1 FROM app_settings WHERE singleton=1",
            [session],
            |r| r.get(0),
        )
        .map_err(Into::into)
}
fn lease_current(connection: &Connection, lease: &AnalysisLease, now: i64) -> Result<bool> {
    let active: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM analysis_job WHERE job_id=?1
        AND state='running' AND session_id=?2 AND lease_token=?3 AND lease_until_ms>=?4)",
        params![lease.job_id, lease.session_id, lease.token, now],
        |row| row.get(0),
    )?;
    Ok(now >= 0
        && active
        && session_current(connection, &lease.session_id)?
        && matches_source(connection, &lease.input)?)
}
fn recover(connection: &Connection, now: i64, session: &str) -> Result<()> {
    let location: Option<String> = connection
        .query_row(
            "SELECT location_id FROM analysis_job
        WHERE state='running' AND (session_id IS NOT ?1 OR lease_until_ms<?2) LIMIT 1",
            params![session, now],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(location) = location {
        let old = cell(connection, &location)?.ok_or(StorageError::InvalidSchema)?;
        let state = if !matches_source(connection, &old.input)? {
            "stale"
        } else if old.attempt < MAX_ATTEMPTS {
            "queued"
        } else {
            "failed"
        };
        clear(connection, &old.job_id, state, Some("INTERRUPTED"), now)?;
    }
    Ok(())
}

impl Database {
    /// Replaces the previous process session. The native host starts one owner
    /// only; all callbacks from an earlier session are fenced out.
    pub fn start_analysis_session(&mut self, now: i64) -> Result<String> {
        if now < 0 {
            return Err(StorageError::InvalidSchema);
        }
        self.transaction(|tx| {
            let session = uuid::Uuid::now_v7().to_string();
            tx.execute(
                "UPDATE app_settings SET analysis_session_id=?1 WHERE singleton=1",
                [&session],
            )?;
            recover(tx, now, &session)?;
            Ok(session)
        })
    }

    /// Visits at most limit location rows, including detached/ineligible rows,
    /// using the primary-key seek. At most 128 pending scheduling cells exist.
    /// Terminal same-input work is not automatically retried on every sweep.
    pub fn discover_analysis_jobs(
        &mut self,
        after: Option<&AnalysisCursor>,
        limit: usize,
        now: i64,
    ) -> Result<Option<AnalysisCursor>> {
        if !(1..=MAX_ANALYSIS_BATCH).contains(&limit) || now < 0 {
            return Err(StorageError::InvalidCursor);
        }
        self.transaction(|tx| {
            let mut statement=tx.prepare("SELECT id,scan_root_id FROM file_location WHERE id COLLATE BINARY>?1 ORDER BY id COLLATE BINARY LIMIT ?2")?;
            let locations=statement.query_map(params![after.map_or("",|c|c.0.as_str()),limit as i64],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            drop(statement);
            let mut pending:i64=tx.query_row("SELECT count(*) FROM analysis_job WHERE state IN ('queued','running')",[],|r|r.get(0))?;
            for (location,root) in &locations {
                let Some(root)=root else {continue;};
                let input=match metadata::capture(tx,root,location) {Ok(input)=>input,Err(StorageError::NotFound)=>continue,Err(e)=>return Err(e)};
                if input.byte_size>MAX_FILE_BYTES {continue;}
                // A valid current snapshot serves the physical project file,
                // including its other aliases. Requiring this candidate's
                // location would make hardlink aliases endlessly reparse.
                let fresh:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM project_file f JOIN metadata_snapshot s ON s.id=f.current_metadata_snapshot_id
                    JOIN file_location l ON l.id=s.input_location_id JOIN scan_root r ON r.id=s.input_root_id
                    WHERE f.id=?1 AND s.project_file_id=f.id AND l.project_file_id=f.id AND l.scan_root_id=r.id
                      AND r.enabled=1 AND r.mode='local_ntfs' AND l.presence='present'
                      AND r.configuration_revision=s.input_root_revision AND f.metadata_revision=s.input_file_revision
                      AND l.metadata_revision=s.input_location_revision AND f.byte_size=s.input_byte_size AND l.byte_size=s.input_byte_size
                      AND f.modified_at_ns=s.input_modified_at_ns AND l.modified_at_ns=s.input_modified_at_ns
                      AND s.adapter_version=?2 AND s.parser_schema_version=?3 AND s.outcome IN ('complete','partial'))",
                    params![input.project_file_id,ADAPTER_VERSION,SCHEMA_VERSION as i64],|r|r.get(0))?;
                if fresh {continue;}
                let old=cell(tx,location)?;
                let same_source=old.as_ref().is_some_and(|old| same_desired_source(&old.input,&input)
                    && old.version==ADAPTER_VERSION && old.schema==SCHEMA_VERSION as i64);
                if old.as_ref().is_some_and(|old| {
                    let unchanged_terminal = matches!(old.state, AnalysisState::Complete|AnalysisState::Unsupported|AnalysisState::Failed|AnalysisState::Cancelled)
                        && same_source;
                    (old.input==input || unchanged_terminal) && old.version==ADAPTER_VERSION && old.schema==SCHEMA_VERSION as i64
                }) {continue;}
                let already_pending=old.as_ref().is_some_and(|old| matches!(old.state,AnalysisState::Queued|AnalysisState::Running));
                if pending>=MAX_ANALYSIS_BATCH as i64 && !already_pending {continue;}
                // Refresh a publication fence without resetting the same
                // source's attempt budget or shortening a queued backoff.
                let (attempt,due)=if same_source {
                    let old=old.as_ref().ok_or(StorageError::InvalidSchema)?;
                    (old.attempt,match old.state {
                        AnalysisState::Queued=>old.due.max(now),
                        AnalysisState::Running=>now.checked_add(1_000).ok_or(StorageError::InvalidSchema)?,
                        _=>now,
                    })
                } else {(0,now)};
                tx.execute("INSERT INTO analysis_job (location_id,job_id,root_id,project_file_id,root_revision,file_revision,location_revision,
                    publication_revision,byte_size,modified_at_ns,adapter_version,parser_schema_version,state,due_at_ms)
                    VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'queued',?13)
                    ON CONFLICT(location_id) DO UPDATE SET job_id=excluded.job_id,root_id=excluded.root_id,project_file_id=excluded.project_file_id,
                      root_revision=excluded.root_revision,file_revision=excluded.file_revision,location_revision=excluded.location_revision,
                      publication_revision=excluded.publication_revision,byte_size=excluded.byte_size,modified_at_ns=excluded.modified_at_ns,
                      adapter_version=excluded.adapter_version,parser_schema_version=excluded.parser_schema_version,state='queued',attempt=?14,
                      due_at_ms=excluded.due_at_ms,session_id=NULL,lease_token=NULL,lease_until_ms=NULL,snapshot_id=NULL,error_code=NULL",
                    params![input.location_id,uuid::Uuid::now_v7().to_string(),input.root_id,input.project_file_id,input.root_revision,
                        input.file_revision,input.location_revision,input.publication_revision,input.byte_size as i64,input.modified_at_ns,
                        ADAPTER_VERSION,SCHEMA_VERSION as i64,due,attempt])?;
                if !already_pending {pending+=1;}
            }
            Ok(if locations.len()==limit {locations.last().map(|(id,_)|AnalysisCursor(id.clone()))} else {None})
        })
    }

    pub fn claim_analysis_job(&mut self, session: &str, now: i64) -> Result<Option<AnalysisLease>> {
        let until = now
            .checked_add(LEASE_MS)
            .filter(|_| now >= 0)
            .ok_or(StorageError::InvalidSchema)?;
        self.transaction(|tx| {
            if !session_current(tx,session)? {return Err(StorageError::Conflict);}
            recover(tx,now,session)?;
            let busy:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM analysis_job WHERE state='running')",[],|r|r.get(0))?;
            if busy {return Ok(None);}
            for _ in 0..MAX_ANALYSIS_BATCH {
                let location:Option<String>=tx.query_row("SELECT location_id FROM analysis_job WHERE state='queued' AND due_at_ms<=?1
                    ORDER BY due_at_ms,job_id COLLATE BINARY LIMIT 1",[now],|r|r.get(0)).optional()?;
                let Some(location)=location else {return Ok(None);};
                let cell=cell(tx,&location)?.ok_or(StorageError::InvalidSchema)?;
                if !matches_source(tx,&cell.input)? || cell.version!=ADAPTER_VERSION || cell.schema!=SCHEMA_VERSION as i64 {
                    clear(tx,&cell.job_id,"stale",Some("SOURCE_CHANGED"),now)?;continue;
                }
                if cell.attempt>=MAX_ATTEMPTS {clear(tx,&cell.job_id,"failed",Some("INTERRUPTED"),now)?;continue;}
                let (root,relative,volume,file_id):(String,String,Option<String>,Option<String>)=tx.query_row("SELECT r.canonical_path,l.relative_path,l.identity_volume_serial,l.identity_file_id
                    FROM file_location l JOIN scan_root r ON r.id=l.scan_root_id WHERE l.id=?1",[&location],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
                let relative=PathBuf::from(relative.replace('\\',std::path::MAIN_SEPARATOR_STR));
                if relative.is_absolute() || relative.components().any(|p|!matches!(p,std::path::Component::Normal(_))) {
                    clear(tx,&cell.job_id,"failed",Some("SOURCE_UNAVAILABLE"),now)?;continue;
                }
                let identity=match (volume,file_id) {(Some(v),Some(f))=>Some((v.parse().map_err(|_|StorageError::InvalidSchema)?,f.parse().map_err(|_|StorageError::InvalidSchema)?)),(None,None)=>None,_=>return Err(StorageError::InvalidSchema)};
                let root=PathBuf::from(root);let path=root.join(relative);
                let token=uuid::Uuid::now_v7().to_string();
                tx.execute("UPDATE analysis_job SET state='running',attempt=attempt+1,session_id=?2,lease_token=?3,lease_until_ms=?4,error_code=NULL WHERE job_id=?1",
                    params![cell.job_id,session,token,until])?;
                return Ok(Some(AnalysisLease {job_id:cell.job_id,session_id:session.to_owned(),token,
                    source:AnalysisSource {root,path,byte_size:cell.input.byte_size,modified_at_ns:cell.input.modified_at_ns,identity},input:cell.input}));
            }
            Ok(None)
        })
    }

    pub fn analysis_lease_current(&self, lease: &AnalysisLease, now: i64) -> Result<bool> {
        lease_current(&self.connection, lease, now)
    }

    /// Commit validated snapshot AND terminal queue state in one writer
    /// transaction while the worker still owns its authorized source guard.
    pub fn complete_analysis_job(
        &mut self,
        lease: &AnalysisLease,
        capabilities: &ParserCapabilities,
        reply: ProtocolReply,
        native_hash: &str,
        now: i64,
    ) -> Result<MetadataSnapshotHeader> {
        self.complete_analysis_observed(lease, capabilities, reply, native_hash, now, || {})
    }

    fn complete_analysis_observed(
        &mut self,
        lease: &AnalysisLease,
        capabilities: &ParserCapabilities,
        reply: ProtocolReply,
        native_hash: &str,
        now: i64,
        before_commit: impl FnOnce(),
    ) -> Result<MetadataSnapshotHeader> {
        self.transaction(|tx| {
            if !lease_current(tx,lease,now)? {return Err(StorageError::Conflict);}
            let snapshot=metadata::publish(tx,&lease.input,capabilities,reply,now,Some(native_hash),||{})?;
            let state=match snapshot.outcome {crate::MetadataOutcome::Complete|crate::MetadataOutcome::Partial=>"complete",crate::MetadataOutcome::Unsupported=>"unsupported",_=>"failed"};
            clear(tx,&lease.job_id,state,None,now)?;
            tx.execute("UPDATE analysis_job SET snapshot_id=?2,publication_revision=publication_revision+1 WHERE job_id=?1",params![lease.job_id,snapshot.id])?;
            before_commit();
            Ok(snapshot)
        })
    }

    pub fn fail_analysis_job(
        &mut self,
        lease: &AnalysisLease,
        failure: AnalysisFailure,
        now: i64,
    ) -> Result<()> {
        if now < 0 {
            return Err(StorageError::InvalidSchema);
        }
        self.transaction(|tx| {
            // Exact control ownership still required when the source is stale.
            let owns:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM analysis_job WHERE job_id=?1 AND state='running' AND session_id=?2 AND lease_token=?3)",params![lease.job_id,lease.session_id,lease.token],|r|r.get(0))?;
            if !owns || !session_current(tx,&lease.session_id)? {return Err(StorageError::Conflict);}
            let attempt:i64=tx.query_row("SELECT attempt FROM analysis_job WHERE job_id=?1",[&lease.job_id],|r|r.get(0))?;
            let state=if !matches_source(tx,&lease.input)? || failure==AnalysisFailure::SourceChanged {"stale"}
                else if failure==AnalysisFailure::Cancelled {"cancelled"}
                else if matches!(failure,AnalysisFailure::Interrupted|AnalysisFailure::ParserTransport) && attempt<MAX_ATTEMPTS {"queued"} else {"failed"};
            let due=now.checked_add(if state=="queued" {1_000} else {0}).ok_or(StorageError::InvalidSchema)?;
            clear(tx,&lease.job_id,state,Some(failure.as_str()),due)
        })
    }

    pub fn cancel_analysis_job(&mut self, job_id: &str, now: i64) -> Result<()> {
        if now < 0 {
            return Err(StorageError::InvalidSchema);
        }
        self.transaction(|tx| {
            let changed=tx.execute("UPDATE analysis_job SET state='cancelled',error_code='CANCELLED',session_id=NULL,lease_token=NULL,lease_until_ms=NULL
                WHERE job_id=?1 AND state IN ('queued','running')",[job_id])?;
            if changed==0 {return Err(StorageError::NotFound);} Ok(())
        })
    }

    pub fn analysis_status(&self, location: &str) -> Result<Option<AnalysisStatus>> {
        self.connection.query_row("SELECT job_id,location_id,state,attempt,snapshot_id,error_code FROM analysis_job WHERE location_id=?1",[location],|r| {
            Ok(AnalysisStatus {job_id:r.get(0)?,location_id:r.get(1)?,state:AnalysisState::parse(&r.get::<_,String>(2)?).map_err(|_|rusqlite::Error::InvalidQuery)?,attempt:r.get(3)?,snapshot_id:r.get(4)?,error_code:r.get(5)?})
        }).optional().map_err(Into::into)
    }

    /// Read only the selected current source's operational status. A location
    /// alone is insufficient: old jobs and another alias must not be presented
    /// as this input's analysis. This performs no scheduling or file access.
    pub fn current_analysis_status(&self, input: &MetadataInput) -> Result<Option<AnalysisStatus>> {
        if !matches_source(&self.connection, input)? {
            return Ok(None);
        }
        let Some(cell) = cell(&self.connection, &input.location_id)? else {
            return Ok(None);
        };
        if !same_desired_source(&cell.input, input)
            || cell.version != ADAPTER_VERSION
            || cell.schema != SCHEMA_VERSION as i64
        {
            return Ok(None);
        }
        let status = self
            .analysis_status(&input.location_id)?
            .ok_or(StorageError::InvalidSchema)?;
        if !(0..=MAX_ATTEMPTS).contains(&status.attempt)
            || (matches!(
                status.state,
                AnalysisState::Running | AnalysisState::Complete | AnalysisState::Unsupported
            ) && status.attempt == 0)
        {
            return Err(StorageError::InvalidSchema);
        }
        if let Some(id) = &status.snapshot_id {
            let snapshot = self
                .metadata_snapshot(id)?
                .ok_or(StorageError::InvalidSchema)?;
            let header = snapshot.header();
            if header.project_file_id != input.project_file_id
                || header.input_root_id != input.root_id
                || header.input_location_id != input.location_id
                || header.input_root_revision != input.root_revision
                || header.input_file_revision != input.file_revision
                || header.input_location_revision != input.location_revision
                || header.input_byte_size != input.byte_size
                || header.input_modified_at_ns != input.modified_at_ns
                || header.adapter_id != fruitboard_flp_parser::ADAPTER_ID
                || header.adapter_version != ADAPTER_VERSION
                || header.parser_schema_version != SCHEMA_VERSION
            {
                return Err(StorageError::InvalidSchema);
            }
            let consistent = match status.state {
                AnalysisState::Complete => matches!(
                    header.outcome,
                    crate::MetadataOutcome::Complete | crate::MetadataOutcome::Partial
                ),
                AnalysisState::Unsupported => header.outcome == crate::MetadataOutcome::Unsupported,
                AnalysisState::Failed => matches!(
                    header.outcome,
                    crate::MetadataOutcome::Failed | crate::MetadataOutcome::Rejected
                ),
                _ => false,
            };
            if !consistent {
                return Err(StorageError::InvalidSchema);
            }
        } else if matches!(
            status.state,
            AnalysisState::Complete | AnalysisState::Unsupported
        ) {
            return Err(StorageError::InvalidSchema);
        }
        Ok(Some(status))
    }
}

#[cfg(test)]
mod tests;
