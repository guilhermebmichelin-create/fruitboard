//! Filesystem-metadata reconciliation for one root, without I/O or persistence.
//!
//! Inputs are boundary-normalized relative paths and qualified local identities.
//! A completed enumeration is necessary but not sufficient for production apply:
//! storage must also fence generation, revision, lease and cancellation atomically.
//! This bounded reference core is not wired to a production scan command.
use std::collections::{BTreeMap, BTreeSet};

/// Qualified physical identity, never a logical project identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Identity {
    pub volume: u64,
    pub file: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub size: u64,
    pub modified_ns: i128,
    pub identity: Option<Identity>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub path: String,
    pub metadata: Metadata,
    pub present: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub path: String,
    pub metadata: Metadata,
}

/// End-of-traversal outcome; early EOF without Complete is never authoritative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Complete,
    Partial,
    Cancelled,
    Offline,
    Denied,
    ResourceLimit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejected {
    Incomplete(Outcome),
    DuplicatePath,
    InvalidPath,
    ResourceLimit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Added(String),
    Modified(String),
    Replaced(String),
    Missing(String),
    Restored(String),
    /// Unambiguous same-run physical continuity evidence; histories stay per path.
    RenameEvidence {
        from: String,
        to: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Deterministic normalized-path order, including retained missing locations.
    pub locations: Vec<Location>,
    pub changes: Vec<Change>,
}

// A small reference slice, not the eventual streaming/staging implementation.
const MAX_RECORDS: usize = 10_000;
const MAX_PATH_BYTES: usize = 32_768;
const MAX_TOTAL_PATH_BYTES: usize = 4 * 1024 * 1024;

fn validate_path(path: &str, bytes: &mut usize) -> Result<(), Rejected> {
    // Boundary owns Windows canonicalization/case semantics. Reject obvious
    // non-relative or non-normalized input here without echoing it in errors.
    if path.is_empty()
        || path.starts_with(['/', '\\'])
        || path.contains([':', '\0'])
        || path
            .split(['/', '\\'])
            .any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(Rejected::InvalidPath);
    }
    *bytes = bytes.saturating_add(path.len());
    if path.len() > MAX_PATH_BYTES || *bytes > MAX_TOTAL_PATH_BYTES {
        return Err(Rejected::ResourceLimit);
    }
    Ok(())
}

/// Produces an all-or-nothing proposal. No file bytes are read or modified.
/// Callers must not publish this proposal without the durable apply contract.
pub fn reconcile(
    previous: &[Location],
    observed: &[Observation],
    outcome: Outcome,
) -> Result<Plan, Rejected> {
    if outcome != Outcome::Complete {
        return Err(Rejected::Incomplete(outcome));
    }
    if previous.len().saturating_add(observed.len()) > MAX_RECORDS {
        return Err(Rejected::ResourceLimit);
    }
    let mut bytes = 0;
    let mut old = BTreeMap::new();
    for location in previous {
        validate_path(&location.path, &mut bytes)?;
        if old.insert(location.path.clone(), location).is_some() {
            return Err(Rejected::DuplicatePath);
        }
    }
    let mut seen = BTreeMap::new();
    for observation in observed {
        validate_path(&observation.path, &mut bytes)?;
        if seen.insert(observation.path.clone(), observation).is_some() {
            return Err(Rejected::DuplicatePath);
        }
    }
    let paths: BTreeSet<_> = old.keys().chain(seen.keys()).cloned().collect();
    let mut locations = Vec::new();
    let mut changes = Vec::new();
    for path in paths {
        match (old.get(&path), seen.get(&path)) {
            (Some(before), Some(after)) => {
                if !before.present {
                    changes.push(Change::Restored(path.clone()));
                }
                match (&before.metadata.identity, &after.metadata.identity) {
                    (Some(a), Some(b)) if a != b => changes.push(Change::Replaced(path.clone())),
                    _ if before.metadata != after.metadata => {
                        changes.push(Change::Modified(path.clone()))
                    }
                    _ => {}
                }
                locations.push(Location {
                    path,
                    metadata: after.metadata.clone(),
                    present: true,
                });
            }
            (None, Some(after)) => {
                changes.push(Change::Added(path.clone()));
                locations.push(Location {
                    path,
                    metadata: after.metadata.clone(),
                    present: true,
                });
            }
            (Some(before), None) => {
                if before.present {
                    changes.push(Change::Missing(path.clone()));
                }
                let mut retained = (*before).clone();
                retained.present = false;
                locations.push(retained);
            }
            (None, None) => unreachable!("union contains only known paths"),
        }
    }
    // Count ALL live aliases, not just added/removed candidates. A surviving
    // alias makes a one-removed/one-added pair ambiguous, not a proven rename.
    let mut old_ids: BTreeMap<&Identity, Vec<&str>> = BTreeMap::new();
    let mut new_ids: BTreeMap<&Identity, Vec<&str>> = BTreeMap::new();
    for location in previous.iter().filter(|location| location.present) {
        if let Some(id) = &location.metadata.identity {
            old_ids.entry(id).or_default().push(&location.path);
        }
    }
    for observation in observed {
        if let Some(id) = &observation.metadata.identity {
            new_ids.entry(id).or_default().push(&observation.path);
        }
    }
    for (id, from) in old_ids {
        if let Some(to) = new_ids.get(id)
            && from.len() == 1
            && to.len() == 1
            && from[0] != to[0]
            && !seen.contains_key(from[0])
            && !old.contains_key(to[0])
        {
            changes.push(Change::RenameEvidence {
                from: from[0].into(),
                to: to[0].into(),
            });
        }
    }
    Ok(Plan { locations, changes })
}

#[cfg(test)]
mod tests;
