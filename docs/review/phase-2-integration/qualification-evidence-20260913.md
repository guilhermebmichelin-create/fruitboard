# P2-11 scanner performance qualification evidence — 2026-09-13

Status: **executed once; non-qualifying for the unchanged warm-reconciliation
target.** This is a focused current-main scanner measurement. It is not an
acceptance of Phase 2, the installed desktop UI, DriveFS, FAT32/cross-volume
identity, or the 100,000-entry private-memory target.

## Approved decision record

The owner-approved setup was recorded before timing and was not re-selected
during execution:

| Decision | Recorded choice |
| --- | --- |
| Fixture | `custom-9995`, seed `0`, exactly 10,000 scanner observations; 9,995 FLP files, 5 alias locations, and 4 other files |
| Canonical manifest | `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a` |
| Source | Current merged `main` only; exact measured SHA `69f27f64f26aa657182a9260cc8e78f28a5838fb` |
| Host profile | `repository-minimum` |
| Window/protocol | Dedicated idle window; one warm-up, 10 measured scans, and 3 cancellation measurements |
| Targets | Unchanged: first discovery `<=30,000 ms`, warm p95 `<=10,000 ms`, cooperative-stop p95 `<=1,000 ms`, working-memory observation `<=128 MiB` on this bounded set |
| Historical experiments | No historical A/B or candidate replay; #108/#112 records remain history |

The decision record is separate from the measured source pin: the decision
defines the fixture/profile/protocol, while the source boundary identifies the
exact merged code that produced the measurements. The accepted 10,005-
observation baseline and its historical manifest hash remain preserved as
overflow-safety evidence.

## Source, tooling, and build

Fetched `origin/main` and the clean dedicated checkout both resolved to
`69f27f64f26aa657182a9260cc8e78f28a5838fb`. The merged runbook, existing
`scripts/run-benchmark.mjs` harness, corrected
`scripts/validate-qualification-report.mjs` validator, and benchmark driver
were verified at that boundary; the harness exposes sanitized diagnostics and
the driver exposes bounded `partial_class` reporting.

After timing completed, documentation-only PR #115 advanced live `main` to
`f811cf3cf6c2e0bc4e3cd161bdcd9e063d53eb35`. The evidence branch was rebased
onto that newer live base for publication; this did not change the measured
source boundary, driver, fixture, or results, and no measurement was rerun.

The release driver was built once for qualification in a fresh isolated target
with:

```text
cargo build --release -p fruitboard-scan-execution --example benchmark --locked
```

| Item | Recorded value |
| --- | --- |
| Node | `v24.20.0` |
| Cargo | `cargo 1.98.1 (797e8a9bc 2026-08-05)` |
| rustc | `rustc 1.98.1 (48a229cea 2026-09-01)` |
| rustfmt | `rustfmt 1.9.0-stable (48a229ceae 2026-09-01)` |
| Target | `x86_64-pc-windows-msvc` |
| Release driver | 2,605,568 bytes; SHA-256 `cb7bb097c087074d49363cca1cfb3441b6e47bfd723983c72f577a5055ba95bc` |
| `Cargo.lock` | `cccb9275fa4951be4b03b6c485df998c7c59d523a8a3735b477d36599284bc95` |
| `rust-toolchain.toml` | `d9fcaa39d559fbf29db5d330392335c24db765b2b489ba75bae6ebb36c1a3783` |
| `tools/toolchain-policy.json` | `e6ac8ce9eb585d3c704c3e8c3c3f3ecb1a42720a58bedfd249a897433c5ea3e1` |

The measured source/harness/validator/driver file hashes and complete build
provenance are in the private `merge-verification.json` and
`release-build-fresh.json` records.

### Measurement-boundary hashes

These hashes were captured before the evidence-branch documentation edits.
The runbook hash therefore identifies the executable runbook used at timing,
while the committed runbook contains only the resulting status correction and
evidence link.

| Artifact | SHA-256 |
| --- | --- |
| Runbook at measurement boundary | `a6ea0b3b65ec580b7f2a848362cf412f55e99327841c88dc6061b09285f12ed5` |
| `scripts/run-benchmark.mjs` | `eba7f3c6688493f40aefeebfba51cbb5874d0e1be3ddbf1e1d974945a07a48a1` |
| `scripts/validate-qualification-report.mjs` | `c48af1d9bc049593b962ed1901ca38b1c4d71307963f70338d6ff2f9cdf61e4d` |
| `crates/scan-execution/examples/benchmark.rs` | `b49490c169ae23194f2128cd70d3cafd138685c7834f9f9fd71b6b526b35b4f2` |
| `fixture/manifest.json` file | `4960c1d67f9da980b42fbbc45a3229286de959ba6dfbbc8e0d8cf48f22c43dc4` |
| Fresh release driver | `cb7bb097c087074d49363cca1cfb3441b6e47bfd723983c72f577a5055ba95bc` |

The build provenance recorded these dependency/tooling inputs: `Cargo.toml`
`0010a6f75a13dd7ed6c5d013e31c5ac45108673943fed23c5d474b801dd8b45b`,
`Cargo.lock`
`cccb9275fa4951be4b03b6c485df998c7c59d523a8a3735b477d36599284bc95`,
`rust-toolchain.toml`
`d9fcaa39d559fbf29db5d330392335c24db765b2b489ba75bae6ebb36c1a3783`,
`package.json`
`6eedb84fbd6ef542d0199b784b96aac704e77ea442d6c71511bfb6dbda031065`,
`pnpm-lock.yaml`
`f7b4189ddb42a18ed5e3dc697babf0ffd2339b962c424ab3c63e35f2ccd41de5`,
`tools/toolchain-policy.json`
`e6ac8ce9eb585d3c704c3e8c3c3f3ecb1a42720a58bedfd249a897433c5ea3e1`, and
`.node-version`
`5b9d0e73029969ae9000117cb877f17bb9841c1279bfe8024e294acfcf017800`.

## Storage and preparation

The initial capacity review found approximately 2.12 GB free on C:, the only
available local NTFS volume; no second local NTFS volume was available. The
usable plan reused already-installed pinned tool artifacts through a directory
junction, kept the fixture and a fresh isolated Cargo target outside the
repository, and performed no deletion. The fresh target was 142,607,127
logical bytes and the fixture was 27,417,589 logical bytes. The private storage
records retain the initial and post-build capacity observations.

## Host evidence

The runbook preflight passed before timing under `repository-minimum`:

| Gate/observation | Result |
| --- | --- |
| Fixture volume | C:, local NTFS |
| Power | AC power passed; High performance was observed but was not a selected control |
| Test lock | Clear; Foundation Smoke and dedicated storage owner locks absent |
| Process inspection | Known; prohibited build/test/benchmark/installed-run activity clear |
| Quiescence | Agent and workload attestations confirmed after the explicit process/lock audit |
| Defender | Readable; real-time protection observed enabled; fixture exclusion not required by this profile |
| DriveFS | Two processes observed; presence-only observation, not a gate under this profile |
| 60-sample CPU observation | Average 4.3833%, minimum 0%, maximum 44%; numerical strict thresholds were not selected |
| 60-sample disk observation | Average 96.3833% idle, minimum 93%, maximum 99%; numerical strict thresholds were not selected |

No security, cloud-sync, or unrelated process settings were changed.

## Attempt dispositions

The harness retained every attempt. All driver exits were `0`; measured scans
were authoritative `Published`/`Complete` records with 10,000 locations, and
all cancellations were terminal `Cancelled` records. No measured attempt was
replaced or discarded.

| Attempt | Status/outcome | Authoritative | Locations | Scan ms | Stop latency ms | Exit |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| warmup | Published / Complete | yes | 10,000 | 9,813 | — | 0 |
| iteration-1 | Published / Complete | yes | 10,000 | 10,025 | — | 0 |
| iteration-2 | Published / Complete | yes | 10,000 | 10,031 | — | 0 |
| iteration-3 | Published / Complete | yes | 10,000 | 9,958 | — | 0 |
| iteration-4 | Published / Complete | yes | 10,000 | 10,003 | — | 0 |
| iteration-5 | Published / Complete | yes | 10,000 | 10,025 | — | 0 |
| iteration-6 | Published / Complete | yes | 10,000 | 9,984 | — | 0 |
| iteration-7 | Published / Complete | yes | 10,000 | 10,021 | — | 0 |
| iteration-8 | Published / Complete | yes | 10,000 | 10,007 | — | 0 |
| iteration-9 | Published / Complete | yes | 10,000 | 9,956 | — | 0 |
| iteration-10 | Published / Complete | yes | 10,000 | 10,173 | — | 0 |
| cancel-1 | Cancelled / Cancelled | no | — | 258 | 8 | 0 |
| cancel-2 | Cancelled / Cancelled | no | — | 259 | 8 | 0 |
| cancel-3 | Cancelled / Cancelled | no | — | 258 | 7 | 0 |

Required sanitized `lines`, `stderrObserved`, and working-set fields were
present for every attempt. The warm-up was reported separately and was not
included in the measured scan sample.

## Budget disposition

| Budget | Derived result | Disposition |
| --- | ---: | --- |
| First discovery | 9,813 ms (`<=30,000`) | PASS |
| Warm reconciliation nearest-rank p95 | 10,173 ms (`>10,000`); median 10,014 ms, max 10,173 ms | FAIL |
| Cooperative-stop nearest-rank p95 | 8 ms (`<=1,000`) | PASS |
| Incremental working-set observation | 53.3 MiB (`<=128 MiB`) on 10,000 observations | PASS; not a 100,000-entry qualification |

The corrected validator returned exit `1` with:

```text
disposition: non-qualifying
harnessExit: 0
measuredAttempts: 10
successfulAuthoritative: 10
authoritativeSampleCount: 10
failedOrNonAuthoritative: 0
validationFailures: warm-reconciliation target missed: nearest-rank p95 exceeds 10,000 ms
```

The independent integrity/statistics check passed. It independently derived
scan values `[9956, 9958, 9984, 10003, 10007, 10021, 10025, 10025, 10031,
10173]`, scan median `10014`, scan p95/max `10173`, cancellation values `[7,
8, 8]`, cancellation median/p95/max `8`, and authoritative sample counts
10 and 3. Reported aggregates matched those derivations.

## Limitations and private artifacts

This result does not claim UI acknowledgement latency, 100,000-entry private
memory, DriveFS/FAT32/cross-volume support, network-share support, or overall
Phase 2 acceptance. It does not explain or relabel the historical #108
`Failed`/`Partial`, and it does not enable production scanning.

The fixture, binary, target, raw report, console log, validator output,
preflight evidence, independent checks, and build logs are retained outside
the repository under private run ID
`fruitboard-qualification-20260913-8c3f0c2a`. The committed branch contains
only this sanitized summary and documentation corrections; the absolute local
artifact path is intentionally omitted here.

During preparation, an initial NTFS hardlink cache view was rejected after
Cargo followed a hardlink and changed the pre-existing ignored target
executable. The pre-experiment hash was `3f8ac1f51f4e26ae8c6fe4bc982674c6eaf9e1dc8244e3b7d99df268ad1b6aec`;
no untouched copy was found in the bounded recovery search. That output was
not used for qualification. The valid driver above came from the fresh target;
the incident and recovery logs remain private and are included in the private
artifact hash index.

No A/B rerun, target change, outlier discard, security-setting change, or
production scan was performed.
