# Windows development parser packaging

Date: 2026-09-30. Related issue: #234. Source baseline: merged PR #233
(`725dd8fe778239196b6a9cde54549c9407b5b663`).

## Delivered boundary

The separate unsigned, current-user Foundation Smoke NSIS package includes
the real Rust parser alongside the inert lifecycle probe. The optional native
`packaging-smoke` feature selects only the fixed installed sibling of the
application executable; missing, non-file and reparse/symlink parser entries
fail closed. The renderer gains no process permission or executable selector.
Default desktop builds do not activate parser jobs or read FLPs.

The installed host validates health and the parser descriptor, reuses its owned
child after an unauthorized request is rejected, and shuts down explicitly.
The harness validates the child's parent, absolute executable path and start
time and retains its process handle before terminating it abruptly. The host
retires that child and successfully starts a replacement. A later health
request also succeeds after explicit shutdown. A separate missing-binary launch
contains the failure without searching for another executable, and the harness
restores the independently verified original bytes.

## Validation

- Locked parser and native `packaging-smoke` tests: 40 desktop and 70 parser
  tests passed. The real parser test covers PID observation, reuse, shutdown
  and restart. Native resolution tests cover missing files, directories,
  relative paths and the fixed sibling.
- Warning-denied all-target Clippy for both changed crates passed.
- Full pinned Windows `pnpm.cmd check` passed, including default native tests,
  repository/client checks and the default release application build.
- Installed Windows AMD64 smoke passed with the exclusive host lock, from
  installer/install paths containing spaces and Unicode. Seed and reinstall
  launches each passed real-parser lifecycle and forced-crash recovery; the
  missing parser launch passed; no owned parser child remained after exit.
- Both uninstalls preserved the 212,992-byte synthetic database byte-for-byte;
  reinstall restored the Library startup preference. Previous synthetic data
  was archived, and current data/evidence remain retained for review.
- Packaging CI now runs locked native feature tests and all-target Clippy
  before its existing installed smoke. Exact-head CI is required before merge;
  local evidence does not substitute for those checks.

The successful package measured 2,840,432 installer bytes and 520,192 parser
bytes. Parser SHA-256:
`b00013f5d3dea96e305d131715bc7d8b68f04a429efe8c8917c365771e4dbd47`.
These are artifact observations, not performance acceptance measurements.

The first installed attempt completed the native parser lifecycle but stopped
in a PowerShell ancestor-path guard. The guard was corrected to use the actual
directory type instead of a PowerShell-added property. Its test installation
was uninstalled under an exclusive lock with the database preserved; raw
evidence and independent executable copies were retained. The complete rerun
passed all cases.

## Evidence and cache handoff

Private raw reports/logs, independent binaries and copied installers remain
outside Git and disposable compiler caches in the retained
`parser-packaging-20260930` evidence directory. Its local handoff records the
absolute paths, source/build provenance, binary/corpus hashes and final storage
preflight. No private paths, FLP bytes, project metadata or databases are
committed in this summary.

Only the existing reusable `bounded-probes` validation cache was written, under
the exclusive ownership of Codex `/root`. The primary development cache was
preserved. Preflight allowed 4 GiB additional output above the 30 GiB reserve;
the final capacity and independently copied Cargo-created internal aliases are
recorded in the local handoff. No compiler tree, worktree, fixture, dependency
or retained evidence was deleted. Reusable compiler output stays in the one
cache; no new permanent cache is introduced.

## Remaining gates

This is unsigned development lifecycle evidence. It does not qualify public
distribution, signing/update rollback, memory/CPU ceilings, descendant-process
containment, broader saved-build compatibility, performance, interactive/audio
behavior or an installed personal-project workflow. The local run used the
noninteractive smoke mode. The next app slice is immutable validated metadata
storage and migrations, followed by native analysis jobs and Library views.
The parser's exact supported builds and approved corpus remain unchanged.
