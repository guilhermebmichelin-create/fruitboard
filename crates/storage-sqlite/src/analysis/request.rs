//! Explicit requests preserve source budgets and never change saved facts.
use super::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisRequestBlock {
    Pending,
    AttemptLimit,
    Unsupported,
    SourceChanged,
    UnqualifiedSource,
    TooLarge,
    QueueFull,
}
// No Debug/Serialize: the opaque key belongs only to the authorized local action.
pub enum AnalysisRequest {
    Ready { key: String },
    Blocked(AnalysisRequestBlock),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisRequestOutcome {
    Queued,
    Blocked(AnalysisRequestBlock),
}

fn same_current(cell: &Cell, input: &MetadataInput) -> bool {
    same_desired_source(&cell.input, input)
        && cell.version == ADAPTER_VERSION
        && cell.schema == SCHEMA_VERSION as i64
}

fn key(input: &MetadataInput, old: Option<&Cell>) -> Result<String> {
    // These are opaque database identities/revisions, never paths. Publication
    // order and the observed job cell also fence a late/replayed action.
    let job = old.map(|old| {
        (
            &old.job_id,
            old.state as u8,
            old.attempt,
            &old.version,
            old.schema,
        )
    });
    let bytes = serde_json::to_vec(&(
        "fruitboard-analysis-request-v1",
        &input.root_id,
        &input.location_id,
        &input.project_file_id,
        input.root_revision,
        input.file_revision,
        input.location_revision,
        input.publication_revision,
        input.byte_size,
        input.modified_at_ns,
        ADAPTER_VERSION,
        SCHEMA_VERSION,
        job,
    ))
    .map_err(|_| StorageError::InvalidSchema)?;
    Ok(fruitboard_flp_parser::sha256_hex(&bytes))
}

fn policy(
    connection: &Connection,
    input: &MetadataInput,
    old: Option<&Cell>,
) -> Result<Option<AnalysisRequestBlock>> {
    if !matches_source(connection, input)? {
        return Ok(Some(AnalysisRequestBlock::SourceChanged));
    }
    if input.byte_size > MAX_FILE_BYTES {
        return Ok(Some(AnalysisRequestBlock::TooLarge));
    }
    let (volume, file): (Option<String>, Option<String>) = connection.query_row(
        "SELECT identity_volume_serial,identity_file_id FROM file_location WHERE id=?1",
        [&input.location_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    match (volume, file) {
        (Some(volume), Some(file)) => {
            volume
                .parse::<u64>()
                .map_err(|_| StorageError::InvalidSchema)?;
            file.parse::<u128>()
                .map_err(|_| StorageError::InvalidSchema)?;
        }
        (None, None) => return Ok(Some(AnalysisRequestBlock::UnqualifiedSource)),
        _ => return Err(StorageError::InvalidSchema),
    }
    if let Some(old) = old.filter(|old| same_current(old, input)) {
        if !(0..=MAX_ATTEMPTS).contains(&old.attempt) {
            return Err(StorageError::InvalidSchema);
        }
        let blocked = match old.state {
            AnalysisState::Queued | AnalysisState::Running => Some(AnalysisRequestBlock::Pending),
            AnalysisState::Unsupported => Some(AnalysisRequestBlock::Unsupported),
            AnalysisState::Stale => Some(AnalysisRequestBlock::SourceChanged),
            _ if old.attempt >= MAX_ATTEMPTS => Some(AnalysisRequestBlock::AttemptLimit),
            _ => None,
        };
        if blocked.is_some() {
            return Ok(blocked);
        }
    }
    let replacing_pending =
        old.is_some_and(|old| matches!(old.state, AnalysisState::Queued | AnalysisState::Running));
    let pending: i64 = connection.query_row(
        "SELECT count(*) FROM analysis_job WHERE state IN ('queued','running')",
        [],
        |r| r.get(0),
    )?;
    Ok((pending >= MAX_ANALYSIS_BATCH as i64 && !replacing_pending)
        .then_some(AnalysisRequestBlock::QueueFull))
}

impl Database {
    pub fn analysis_request(&self, input: &MetadataInput) -> Result<AnalysisRequest> {
        let old = cell(&self.connection, &input.location_id)?;
        Ok(match policy(&self.connection, input, old.as_ref())? {
            Some(blocked) => AnalysisRequest::Blocked(blocked),
            None => AnalysisRequest::Ready {
                key: key(input, old.as_ref())?,
            },
        })
    }

    /// Key, source, eligibility and queue mutation share one IMMEDIATE writer
    /// transaction. A replay cannot shorten backoff or reset started attempts.
    pub fn request_analysis_job(
        &mut self,
        input: &MetadataInput,
        expected_key: &str,
        now: i64,
    ) -> Result<AnalysisRequestOutcome> {
        if now < 0 {
            return Err(StorageError::InvalidSchema);
        }
        self.transaction(|tx| {
            if !matches_source(tx, input)? { return Err(StorageError::Conflict); }
            let old = cell(tx, &input.location_id)?;
            if key(input, old.as_ref())? != expected_key { return Err(StorageError::Conflict); }
            if let Some(blocked) = policy(tx, input, old.as_ref())? {
                return Ok(AnalysisRequestOutcome::Blocked(blocked));
            }
            let attempt = old.as_ref().filter(|old| same_current(old, input)).map_or(0, |old| old.attempt);
            tx.execute("INSERT INTO analysis_job (location_id,job_id,root_id,project_file_id,root_revision,file_revision,location_revision,publication_revision,byte_size,modified_at_ns,adapter_version,parser_schema_version,state,due_at_ms)
                VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'queued',?13)
                ON CONFLICT(location_id) DO UPDATE SET job_id=excluded.job_id,root_id=excluded.root_id,project_file_id=excluded.project_file_id,
                root_revision=excluded.root_revision,file_revision=excluded.file_revision,location_revision=excluded.location_revision,
                publication_revision=excluded.publication_revision,byte_size=excluded.byte_size,modified_at_ns=excluded.modified_at_ns,
                adapter_version=excluded.adapter_version,parser_schema_version=excluded.parser_schema_version,state='queued',attempt=?14,
                due_at_ms=excluded.due_at_ms,session_id=NULL,lease_token=NULL,lease_until_ms=NULL,snapshot_id=NULL,error_code=NULL",
                params![input.location_id,uuid::Uuid::now_v7().to_string(),input.root_id,input.project_file_id,input.root_revision,
                input.file_revision,input.location_revision,input.publication_revision,input.byte_size as i64,input.modified_at_ns,
                ADAPTER_VERSION,SCHEMA_VERSION as i64,now,attempt])?;
            Ok(AnalysisRequestOutcome::Queued)
        })
    }
}
