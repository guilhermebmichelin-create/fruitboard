# Native analysis execution

Issue #238 connects durable native file observations to the supervised Rust
parser and immutable validated metadata storage. The desktop's explicit
`analysis-jobs` development feature enables it; the default desktop does not.
There is no renderer queue, parser selection, filesystem authority, or new IPC.

## Bounds and authority

Storage visits at most 128 location rows per seek page and retains at most 128
queued/running cells. A cell holds the desired source for one location, with a
new opaque job ID when its revisions or parser version change. A partial unique
index permits one running job globally. A 60-second lease and rotating process
session fence old completions. Interrupted/transport attempts retry after one
second, at most three attempts; an explicit cancel or unchanged terminal failure
survives restart without an automatic retry loop. Changed sources supersede it.
Terminal cells remain operational state, not an append-only attempt history.

The worker releases the database mutex before filesystem or parser operations.
Its Windows authority opens the selected file and all ancestors without
reparses, verifies fixed local NTFS and the recorded volume/file identity, and
rejects offline/recall attributes before content reads. Legacy observations
without qualified identity fail closed until an authoritative scan supplies it.
Only files within the selected enabled `local_ntfs` root and the existing 4 MiB
parser cap qualify. Drive virtual roots never qualify. Other platforms use
portable injected ports in tests and have no native content-reading authority.

Native reads independently hash the held object before and after parsing and
check its nanosecond modification time, length, and identity. Windows handles
deny writing/deleting the file and ancestors until commit. They do not freeze
every possible privileged filesystem operation; this is bounded freshness
validation, not a general filesystem security sandbox. The child also applies
its existing authorized-input checks. Raw sample/plugin references remain inert.

The host resolves only the fixed installed sibling parser, pins its authorized
file/ancestor handles, and uses the existing supervisor's 10-second request
deadline, cancellation, retirement, restart and 256-request recycling. Health
and descriptor validation precede every parse. A scoped 25 ms monitor cancels
work after durable source/root/session/lease revocation; shutdown retires the
owned parser before another host can start. There is one worker, not a pool.

One SQLite IMMEDIATE transaction rechecks the exact lease and source/publication
fences, validates the reply against the independently observed digest, appends
the snapshot, and records the terminal job with its snapshot reference.
`complete`/`partial` results may become current. Negative outcomes preserve a
still-fresh last good snapshot; invalid replies and transport failures cannot
become metadata. Private paths and payloads have no automatic logging or IPC.

## Validation and development package

Use the pinned toolchain and storage/cache coordination in
[DEVELOPMENT.md](../../DEVELOPMENT.md) before these checks:

```text
cargo test -p fruitboard-storage --features analysis-jobs --locked
cargo test -p fruitboard-analysis-execution --locked
cargo clippy -p fruitboard-analysis-execution --all-targets --locked -- -D warnings
cargo test -p fruitboard-desktop --features analysis-jobs --locked
cargo clippy -p fruitboard-desktop --features analysis-jobs --all-targets --locked -- -D warnings
pnpm.cmd check
pnpm.cmd smoke:windows:foundation:hosted -AnalysisJobs
```

Windows CI separately builds the parser, sets `FRUITBOARD_ANALYSIS_TEST_PARSER`
to that exact debug executable and runs the ignored `native_analysis` test.
That test copies the approved fixture, verifies opened-object write/rename
denial, parses through the real process, and checks persisted metadata and
unchanged source bytes. The opt-in installed smoke exercises real native
enumeration, durable queue, parser and publication, then checks the same
snapshot ID/digest after reinstall and byte preservation at both uninstalls.
The baseline installed smoke retains its stricter unchanged-database relaunch
assertion; the opt-in variant legitimately changes native session records.

Portable tests cover queue bounds, stale/cancel/session/lease fences, retries,
alias reuse, invalid digest rejection, SQL rollback and process termination
after both snapshot/job writes but before their shared commit. Queue backup
and recovery are tested from schema 10 to 11. No new corpus data is committed.
Metadata presentation, explicit retry controls, production enablement,
compatibility coverage, signing/distribution, and performance qualification
remain later slices. This implementation makes no throughput claim.
