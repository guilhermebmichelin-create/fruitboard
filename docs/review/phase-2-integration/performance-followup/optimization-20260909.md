# Filesystem enumeration optimization evidence

Status: draft optimization evidence, captured 2026-09-09. This work is stacked
on draft [PR #93](https://github.com/guilhermebmichelin-create/fruitboard/pull/93),
which is still the source of the `profile-fs-calls` diagnostic example and its
original report. This work changes only the filesystem-enumeration crate; it
does not change scan-execution or storage production code, quotas, fixture
definitions, budgets, durability settings, or acceptance criteria.

## Decision at a glance

The safe candidate is to remove one duplicate native attribute query from
`read_handle_metadata`. `FILE_BASIC_INFORMATION.FileAttributes` already
contains the attribute flags used by this boundary, including the reparse and
recall/offline flags; the former `FILE_ATTRIBUTE_TAG_INFORMATION` result was
only ORed into those same flags, and its tag was never consumed.

The candidate preserves every ancestor validation, relative handle open,
identity query, reparse-point fence, root check, cancellation check, locator
key, and authoritative-outcome rule. It does not cache or skip a security
check.

On three same-process native diagnostic runs, entry-metadata time fell from a
median 106.161 ms to 77.649 ms per direct traversal (−26.9%). Total direct
enumeration fell from a median 8,942 ms to 8,728 ms (−2.4%). The native
diagnostic is instrumented and does not include staging or publication; its
numbers must not be compared directly with the whole-scan benchmark.

The required whole-scan ten-iteration comparison completed on both commits,
but the shared host was contended. The candidate median moved from 11,735.5 ms
to 11,517 ms (−1.9%), while one host outlier raised candidate max/nearest-rank
p95 from 12,188 ms to 17,682 ms. This is not an end-to-end p95 win and does
not qualify the 10-second target. A truly quiet-host rerun remains the next
experiment.

## Provenance and ownership

| Item | Before | Candidate |
| --- | --- | --- |
| Repository commit | `1d7c298` | `b35c01a` |
| Diagnostic parent | PR #93 head `59faefc` | PR #93 head `59faefc` |
| Native profile | `profile-native-ops-v1` | `profile-native-ops-v1` |
| Whole-scan driver | uninstrumented default `benchmark` path | same |
| Rust | pinned 1.98.1 MSVC | pinned 1.98.1 MSVC |
| Node | pinned 24.20.0 | pinned 24.20.0 |

The existing PR #93 `profile-fs-calls-v1` wrapper was reused unchanged for
public filesystem-port boundary timing. The new
[`profile-native-ops.rs`](../../../../crates/filesystem-enumeration/examples/profile-native-ops.rs)
is an opt-in `diagnostics` feature owned by this crate. It records native
operation categories without paths or metadata values. The production default
does not enable that feature.

The raw native records are
[`profile-native-before-raw.jsonl`](profile-native-before-raw.jsonl) and
[`profile-native-after-raw.jsonl`](profile-native-after-raw.jsonl). The
uninstrumented whole-scan records are
[`benchmark-before-raw.json`](benchmark-before-raw.json) and
[`benchmark-after-raw.json`](benchmark-after-raw.json).

## Identical fixture and toolchain

Both comparisons used the persistent generated fixture:

| Field | Value |
| --- | --- |
| Size label | `custom-9995` |
| Seed | `0` |
| Manifest SHA-256 | `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a` |
| FLP-named files | 9,995 |
| Alias locations | 5 |
| Total observations | 10,000 |
| Leaf directories | 1,000 |
| Directories | 1,025 |
| Hardlink support | `created` |

The fixture contains synthetic bytes only. The before and candidate binaries
used the same locked workspace dependencies and the same fixture manifest.

## Diagnostic separation

The native profile separates the measured public operations into ancestor
validation, `NtQueryDirectoryFile`, entry open, entry metadata, child-directory
open/metadata, case-sensitivity query, and root work. Three runs were made in
each side after the sibling build/installed-run window had cleared; DriveFS,
Defender, Epic, T3, and Codex activity remained, so these are contended-host
diagnostics.

| Direct traversal metric | Before median | Candidate median | Change |
| --- | ---: | ---: | ---: |
| Enumeration wall time | 8,942 ms | 8,728 ms | −2.4% |
| Timed native-call sum | 8,699 ms | 8,483 ms | −2.5% |
| Ancestor validation | 7,225 ms | 7,038 ms | −2.6% |
| Entry metadata | 106.161 ms | 77.649 ms | −26.9% |
| Ancestor-validation calls | 26,163 | 26,163 | unchanged |
| Validated ancestor links | 60,973 | 60,973 | unchanged |
| Directory-query calls | 14,108 | 14,108 | unchanged |
| Entry opens | 11,030 | 11,030 | unchanged |
| Entry metadata calls | 11,030 | 11,030 | unchanged |
| Observations / outcome | 10,000 / Complete | 10,000 / Complete | unchanged |

The diagnostic result isolates the change to metadata work. It does not show a
reduction in ancestor checks, and it is not evidence that ancestor validation
can be cached.

## Whole-scan ten-iteration protocol

The documented protocol was run with one warm-up, ten measured fresh-process
iterations, three fresh cancellation measurements, and 100 ms working-set
sampling. Both reports used the high-performance power plan and the same
machine class: Intel i7-10750H, 12 logical cores, Windows build 10.0.26200,
x64.

| Metric | Before | Candidate |
| --- | ---: | ---: |
| Warm-up | 11,915 ms | 11,197 ms |
| Measured iterations | 12,019, 12,021, 11,766, 11,986, 11,639, 11,609, 11,627, 12,188, 11,647, 11,705 ms | 11,315, 13,338, 14,453, 17,682, 11,348, 11,377, 11,392, 11,559, 11,475, 16,732 ms |
| Median | 11,735.5 ms | 11,517 ms |
| Max / nearest-rank p95 | 12,188 ms | 17,682 ms |
| Cancellation samples | 5, 6, 5 ms | 6, 6, 7 ms |
| Cancellation max / p95 | 6 ms | 7 ms |
| Measured working-set increment | 53.0–53.1 MiB | 52.9–53.1 MiB |
| Authoritative measured runs | 10 / 10 | 10 / 10 |

The candidate's median and first-run movement is directional but not a quiet-host
performance claim. The p95 remains a failure against the unchanged provisional
`<= 10 s` budget on this host class. No iteration was filtered.

Host classification for both protocol runs is **contended shared-host
evidence**: no cargo/rustc/desktop process was present at each preflight, but
DriveFS, Defender, Epic, T3/Codex tooling, and other agent activity remained.
The run therefore records the result honestly without calling it an idle-host
qualification.

## Safety evidence

- `validate_ancestors` is unchanged and still runs before every metadata,
  child-open, and directory-query operation. Its call and validated-link counts
  are identical before and after.
- Relative file and directory opens still use the parent handle,
  `FILE_OPEN_REPARSE_POINT`, the same read/list access, and the same sharing
  flags. The candidate does not reopen through a constructed path.
- The opened-child identity comparison, bounded validation chain, final root
  identity check, cancellation points, batching, locator-key generation, and
  conservative incomplete outcomes are unchanged.
- The focused native regression
  `ntfs_fixture_metadata_marks_a_junction_as_reparse` opens a real NTFS
  junction through the handle-bound cursor and verifies that metadata still
  reports a directory reparse point. The existing fake-port regressions cover
  root replacement, child replacement between metadata/open, ancestor
  replacement before the next entry, reparse exclusions, cancellation, and
  non-authoritative failure handling.
- The full native fixture suite still preserves hardlink identity semantics,
  Unicode/long locators, marker bytes, and junction non-authority. No FLP
  content is read or changed.

The attribute source is documented by Microsoft's
[`FILE_BASIC_INFO`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_basic_info)
and
[`FILE_ATTRIBUTE_TAG_INFO`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_attribute_tag_info)
definitions: both expose file attributes, while the latter additionally
exposes a reparse tag. This code needs only the attribute flags; it never used
the tag. The implementation continues to require successful basic and
standard handle metadata and retains all identity/reparse checks.

## Checks

Passed on the pinned Windows toolchain:

- `cargo fmt --all -- --check`
- `cargo test -p fruitboard-filesystem-enumeration --locked` — 44 passed,
  2 ignored
- `cargo clippy -p fruitboard-filesystem-enumeration --all-targets --locked -- -D warnings`
- `cargo clippy -p fruitboard-filesystem-enumeration --features diagnostics --example profile-native-ops --locked -- -D warnings`
- release builds for both native diagnostics and the PR #93 profile/benchmark
  driver
- the documented whole-scan protocol on both commits
- `git diff --check`

The full `pnpm check` is not claimed: it includes the shared desktop build,
which is owned by another agent and was active during this wave. The focused
crate checks and the release driver builds above are the equivalent evidence
for this filesystem-only change.

## Next experiment and non-goals

The next experiment is an owner-coordinated quiet-host A/B rerun: pause
DriveFS, apply the documented fixture-volume antivirus exclusion, stop other
agent/build activity, keep AC/high-performance power, and run the same one
warm-up plus ten measured iterations for both commits. Report all samples,
median, max, and nearest-rank p95. If the native benefit survives but whole-scan
p95 remains above 10 seconds, the next isolated measurement should split
staging, reconciliation reads, and publication; it must not weaken SQLite
durability or cache away ancestor validation.

This change does not alter the 10,000-record quota, accepted fixture
definition, 10-second target, 100,000-entry scope, batch limits, cancellation,
identity semantics, or SQLite durability settings.
