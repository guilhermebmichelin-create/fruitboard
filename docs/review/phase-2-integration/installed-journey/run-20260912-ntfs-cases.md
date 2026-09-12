# Installed local NTFS safety cases — 2026-09-12

Status: **Evidence only.** This record does not merge code, modify normal user
data, promote a Phase 2 acceptance ID, or qualify a filesystem other than the
disposable local NTFS volume exercised below.

## Scope, ownership, and preserved history

- Live `origin/main` was fetched and verified as
  `3ebac5f7a76c3425620ceba59e6078b32fb6cd85`, parent
  `b2fb62c36a057985ab0eba02458f037fcb96c215`. The dirty shared `main`
  checkout was not switched, reset, or edited.
- This isolated worktree is branch `test/phase2-ntfs-cases-20260912`.
  `de396e7` preserves the recovered #107 test/report patch from historical
  commit `19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`; `df29c88` adds the
  reproducible harness and evidence tooling; `05fb35c` is the separate
  focused product correction described below. The installed J build used
  source commit `05fb35c153dd3f177b900292d39998da3774b5e4`.
- The September 12 queued/restart experiment is not repeated here. Its
  historical report remains
  [`run-20260912-queued-restart.md`](run-20260912-queued-restart.md), with
  historical driver `queued-state-restart.mjs` SHA-256
  `BC1D3C342A4ADE84EFA473B851AC7A82E302A12A37E6B448844F062C8886BC1D`
  and historical transcript SHA-256
  `0C044A48750B4405B647A78599CE27C59DE7B1C5EF977F5F6BAC56A6F8CBD04F`.
  Those hashes are intentionally distinct from the revised driver and J
  transcript below.
- Agent 3's performance work supplied preparation/preflight evidence only:
  its quiet-host conditions were not met, so no quiet performance window or
  performance qualification is claimed here.

## Reproducible orchestration and lock ownership

The follow-up uses [`run-installed-ntfs-cases.ps1`](../../../scripts/run-installed-ntfs-cases.ps1)
and the existing [`foundation-smoke-lock.ps1`](../../../scripts/foundation-smoke-lock.ps1).
The wrapper requires `-JourneyRoot`; optional tool-path arguments select the
pinned Node, Python, uv, and Rust binaries. It validates that the run root is
a new child of `%TEMP%`, installs beneath that root, and invokes the revised
driver with exactly four required arguments:

```text
<journey-root> <app-path> <lock-path> <run-id>
```

Before archiving shared Foundation Smoke data, building, installing, launching,
or accessing the database, the wrapper acquires the existing exclusive lock.
The driver then fails closed unless the lock JSON matches the exact run ID,
owner PID, purpose, run root, and install directory, and the owner PID is live.
It also refuses an existing transcript, a missing app, a wrong data-directory
basename, or a non-NTFS volume. The J lock was:

| Field               | Value                                                                                                                                                                                                                                          |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Run root            | `%TEMP%/fruitboard-ntfs-cases-run-20260912-j`                                                                                                                                                                                                  |
| Run ID              | `d55cbc9a2faf41b1b48a565f29f3c228`                                                                                                                                                                                                             |
| Owner PID           | `21068`                                                                                                                                                                                                                                        |
| Purpose             | `installed-ntfs-cases-20260912`                                                                                                                                                                                                                |
| Lock helper         | `312F0794FF4C96094B590394AD4D4CF496C7E019BDD96714EE517421FD1DF3A1`                                                                                                                                                                             |
| Prior-data handling | Existing dedicated data was moved reversibly to `archived-previous-foundation-smoke-d55cbc9a2faf41b1b48a565f29f3c228`; its retained database was 16,052,224 bytes, SHA-256 `E8FED066232B8DAC7A1259B0CD1BC056B802688BB382931BF5BA63F4CE891CDF`. |

The wrapper held ownership through build, install, all app launches, evidence
capture, cleanup, and owner-only release. The prior archive and all J run
artifacts remain under `%TEMP%` for review.

## Build and artifact provenance

J was a recorded unsigned NSIS build with `packaging-smoke,scan-console`:

```text
node scripts/prepare-foundation-sidecar.mjs; pnpm.cmd --filter @fruitboard/desktop exec tauri build --ci --no-sign --config src-tauri/tauri.package.conf.json --features packaging-smoke,scan-console --bundles nsis
```

The revised driver is [`installed-ntfs-cases.mjs`](installed-ntfs-cases.mjs),
45,666 bytes, SHA-256
`1885BC829D9FE636B1682BD5FBD3F1F6901D45422E62F8491EF3CCDB316A63A6`.
The wrapper SHA-256 is
`91F4AC6CAFB885D1844D7E5890FF4CE364F9E2ED09EDBD42466BBE7AD55539EC`.
The read-only database inspector
[`inspect-foundation-smoke-db.py`](../../../scripts/inspect-foundation-smoke-db.py)
is 6,359 bytes, SHA-256
`114D73CD4438F7CAD29267DDE113E433081723D261B51BF171ED16A67A9C0579`.

| Item              |            Version / bytes | SHA-256                                                            |
| ----------------- | -------------------------: | ------------------------------------------------------------------ |
| Node              |    `v24.20.0` / 93,381,448 | `5C976096E04E5C2C1F091938926234CC9FBFE9787DDD149351B3B0ECC707B5`   |
| pnpm              |          `11.25.0` / 1,464 | `FF3224D46B47FBB24A7E9FE15FEDEDEF7E00892D07D4E376B6762D4899906BFD` |
| Corepack          |             `0.36.0` / 174 | `3655BC798F300951F2070FEE411B337D626B0C3AE80C2D24C46CCAC4595D4BF9` |
| Rust/cargo/rustup | `1.98.1` / 12,721,664 each | `6F4BEF66261261FCB43131BE8720BAB817D403A09EDEC7455C371974B90BDB7E` |
| uv                |      `0.12.9` / 41,386,496 | `B5D230C79FFA3629422F48BFCE0766E9827769608A79BBB4E4E540081F59D97C` |
| Python            |         `3.11.16` / 45,568 | `44576007DD76F1CB275E0A59B3DE6CC2948E909BBD2527B8E616D100EDEED50E` |
| `Cargo.lock`      |                    120,322 | `CCCB9275FA4951BE4B03B6C485DF998C7C59D523A8A3735B477D36599284BC95` |
| `pnpm-lock.yaml`  |                    107,640 | `F7B4189DDB42A18ED5E3DC697BABF0FFD2339B962C424AB3C63E35F2CCD41DE5` |
| NSIS installer    |                  2,938,144 | `3D203B6162F9DD9370E3FC210C6371ED064687E9F3B03E6D712686E29FE57879` |
| Installed app     |                  9,876,480 | `8CD1F84F0DC67524E7B610A7273CE644F50222342ED2F0BE08A8831328C6E41F` |
| Installed sidecar |                    156,160 | `A5B926DEDF9ACF40F0246D1B3E9A9B4164B012075446C772857CD0CE8CD3D72D` |

The raw evidence artifacts are retained at the J run root. The JSONL
transcript is 126,303 bytes, SHA-256
`C131757566C33E347C59472C723085C632FA6236FB94DE64D86C037F9233E477`;
`driver.stdout.txt` is 253,010 bytes, SHA-256
`BA23E88836ADD0B420D4843B772313B72CAE42C6380639F53DC83E7D57A7BF96D`;
`provenance.json` is 7,256 bytes, SHA-256
`D622C329E95F264CF87FA6B8871567E4E5E7A72227A490DB33399A6997AC3D17`.

## Synthetic data and filesystem boundary

The unmodified [`generate-synthetic-tree.mjs`](../../../scripts/generate-synthetic-tree.mjs)
was used with seed `20260912`; its source SHA-256 is
`1F4F6D82A736027BCD68486019DB12F1EDEAF8BB947B2BA38FE6030D16E190D6`.
The accepted baseline manifest was retained unchanged: 10,000 FLP-named
files, 4 other files, 5 alias locations, and 5 hardlink groups. Its generator
manifest SHA-256 is
`7B04EF5F87334D48602D6D310D25A00480B8F298BEB0F9DCA478F07EB27F04EC`, and
the persisted `fixtures/resource-baseline/manifest.json` is 1,904,257 bytes,
SHA-256 `366A9E27D82B508436664F4FFECEB81CE858D3FA42C406A629455CDE39F9F994`.
The scanned `fixtures/resource-baseline/roots` subroot has 8,000 primary
locations so the committed baseline fits under the existing 10,000-record
bound; the accepted fixture and budget were not changed.

The disposable scenario manifests were synthetic marker bytes only:

| Scenario          | Entries | Manifest file SHA-256                                              | Canonical manifest SHA-256                                         |
| ----------------- | ------: | ------------------------------------------------------------------ | ------------------------------------------------------------------ |
| Denied traversal  |       4 | `D924FD6DAC974697E5E99B2FF98535F8EACFF7932E16B2A72B3E2D969B8D9771` | `DEFBDCEC7D854585093E4AA2AC6EA081ED52D48505757A87C64742FDC8476070` |
| Hardlink aliases  |       2 | `CDA5EE9F200A33815E810719CA12C16E40B00F899AD6EA1C0B857758159FDDE5` | `370F566171034DC0795A9AD370D619F8D2F3FBE700FDBBD54D037BDF61AF3C28` |
| Resource overflow |   2,001 | `1142AF0E651228957431FFF7F8BA90CEA065DFEE8CEDFD5613E936969E890342` | `B75F72700096DE32C5A245C9481520D70E9C86893FC4C2ADDE8E827B57606D84` |

The resource case added 2,001 files under the scanned root, for 10,001
expected observations. The native volume proof was:

```json
{
  "DriveLetter": "C",
  "FileSystem": "NTFS",
  "FileSystemLabel": "Windows-SSD",
  "HealthStatus": "Healthy"
}
```

This evidence makes no claim about FAT32, DriveFS, network shares, cross-volume
identity, or real OS buffer-overflow timing.

## Observations, separated by evidence class

### Denied traversal

Native calls applied `icacls ... /deny guilhermebougle\guilh:(OI)(CI)(RX)`
to the disposable `blocked` subtree after a fresh committed baseline. The
same-user direct Node `readdirSync` probe failed with `EPERM`; the installed
scan then completed its retry chain at `failed/access_denied` with
`retryAvailable=false`. `icacls ... /remove:d ... /T /C` exited 0 and the
recorded restore result was `restored=true`.

The UI snapshot separately showed “The folder could not be read. Previous
committed results were kept.” Its page still contained all 4 prior locations,
all `present`. Database evidence retained the 4-row location digest
`651dbf23e0077eb0b5ee6dbd40df1bf52300196c6b41832518a027e350834619`; the
four failed stages were `discarded` with no `published_at_ms`, and the fresh
last-success snapshot marker remained unchanged.

### ResourceLimit

The UI snapshot separately showed “The scan reached a safe resource limit.
Previous committed results were kept.” The final status was
`failed/resource_limit`, `retryAvailable=false`; the durable job reached
attempt 4/4 with `last_error_code=resource_limit`. The four failed runs also
recorded `resource_limit` (the first run was interrupted by the normal
follow-up request, then attempts 1–4 of the terminal chain failed at the
bound). All failed stages were discarded, with observed staging counts of
2,048 and then 9,728 per attempt and no publication.

The database retained 8,000 `present` rows with unchanged location digest
`43841f2450ae204c1f4b3f3e6d30a6d86b23ded1299bb0ce6476944d37a8cb0d`; the
fresh last-success marker remained at generation 3. No false missing
transition or partial publication occurred. Run G had reproduced the
pre-fix defect in which these same durable failures became `worker_failed`
and the installed UI degraded to `internal`; focused commit `05fb35c` now
propagates only fixed worker-owned diagnostic codes. J verifies the repaired
`resource_limit` result end to end without changing the accepted fixture or
bound.

### Hardlink aliases

Native `fs.stat` evidence showed the backing file and both in-root aliases
were true hardlinks: device `406848235`, file ID `281474978946248`, and 73
bytes for each. `fsutil hardlink list` initially listed backing, alias A, and
alias B. After removing alias A, the surviving native list contained backing
and alias B; after restoration it again contained all three paths.

The UI/native terminal observations were `completed` with two rows both
times: after removal, alias A was `missing` and alias B `present`; after
restoration both were `present` with their distinct location IDs. The final
database's two identity records retained distinct relative paths and locator
keys while sharing the underlying project file identity. The hardlink
location digest remained `0dda6b49f7e0126cbb288d96f33a4366738bf30c377cb130dfcdb90066a5ee9c`.

## Database snapshots and cleanup

The pinned read-only inspector reported `PRAGMA integrity_check = ok` and
`journal_mode = delete` for every snapshot. Database and inspector-evidence
hashes are:

| Snapshot            | Database bytes / SHA-256                                                        | Evidence JSON bytes / SHA-256                                                  |
| ------------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Baseline            | 14,934,016 / `2040B425629DE47D948CD2BE8DD8992082B26E12A181654F0D9C4FDC9883A9AE` | 3,428,813 / `C3A06761CFC0F109C6BF3B4AC253E090D2A67D773BD5EA66018A18665526CD4D` |
| After denied        | 14,934,016 / `B3B72E7EE395E7BDB66C8DB947FB81C5C78383B8DC3D55095340618ADCD97FD3` | 3,438,152 / `BA53A6B9A3266F789ADEF7940C40F22709F48F3EDC4B70D0BBCDE277C8A133C0` |
| After ResourceLimit | 16,052,224 / `D9BCB5DEC425302B1AC13FD379A169F9BC2363BA9CD506DBDA385ACA745D9245` | 3,447,020 / `ED3F16D7A2D0E1A35AC2796AB20AAF30587668C7D3710A5123D297981D4EB97F` |
| Final               | 16,052,224 / `29D029B9AB0E19CEF1577C7CE8046EA8FF8612B1187C91CE4E50F165A6A61B98` | 3,452,031 / `FE0B2255CFC71B8B021EC74D12BFEF9FFF55113EE466FDFBB831DD3D077065AC` |

The final driver result was `pass=true` for baseline commits, denied
traversal, ResourceLimit, and hardlink aliases. The app was stopped, the
run-specific package was removed, the lock was absent, and no
`fruitboard-desktop` process remained. ACL restoration was recorded as
successful and alias A was restored. The prior dedicated-data archive, J
fixtures, transcripts, package hash, and database copies remain available;
normal `com.fruitboard.desktop` data was not accessed.

## Remaining gaps and handoff

- No quiet-host performance qualification: Agent 3's preflight did not meet
  the required DriveFS/antivirus/sibling-activity conditions.
- No FAT32, DriveFS, network-share, cross-volume, or real buffer-overflow
  qualification was attempted or inferred from NTFS.
- F1's accepted 10,005-observation fixture-versus-10,000-record-bound issue,
  F2 quiet-host performance decision, and F3 scale decision remain owner
  decisions. This run changed no fixture or budget.
- Phase 2 acceptance remains pending; this report is evidence for review only.
