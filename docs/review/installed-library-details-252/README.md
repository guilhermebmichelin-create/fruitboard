# Installed Library metadata workflow

Issue [#252](https://github.com/guilhermebmichelin-create/fruitboard/issues/252).
Executed on Windows on 2026-10-03 against merged PR #251, application source
`2d5c769ba76afe182d9ce9141445348c845f687a`. Its tree matches the reviewed
`980d917fc67f2ac6fd6aeabd5a0faed4a49a2d32` tree.

**Result: the bounded installed Library workflow passes.** The real installed
renderer, native commands, scanner, parser and SQLite owner work together for
the approved cases below. No application change was needed. This report adds
installed evidence to the earlier browser and command tests; owner review and
manual merge remain the delivery gate.

## Candidate and observation boundary

The pinned Windows command was `pnpm package:windows:analysis-smoke`. It builds
the matching parser and an unsigned NSIS development installer with
`analysis-jobs` enabled. The test identity is
`com.fruitboard.desktop.foundation-smoke`; normal Fruitboard data is separate.
The installed scan-console command reported both enabled and runtime available.
The production default remains disabled.

WebView2 154.0.4258.53 served the production desktop entry at
`http://tauri.localhost`. Playwright attached to that installed WebView through
its debugging connection. The Library explicitly reported the native adapter.
No browser review adapter or fabricated native response supplied these results.

Approved local NTFS fixture folders were admitted through the actual
`add_scan_root` command. The Windows folder-picker dialog was not exercised
again. Selection, scan, details, refresh, analysis requests, root toggles and
confirmed removal used the rendered controls. Keyboard events were delivered
through WebView debugging, rather than Windows hardware input automation.

## Verified workflow

| Case | Installed observation |
| --- | --- |
| Scan and paging | Visible Scan now published seven approved fixture copies; both Library pages exposed their details through the real read command. |
| Known values | Versions, base tempos, channel labels, instruments and saved references matched the existing fixture expectations listed below. |
| Provenance and limits | Extracted, inferred, unsupported and unavailable fields remained distinct. Partial coverage and unchecked sample/plugin availability were stated explicitly. |
| Read-only refresh | Refresh preserved snapshot identity and all returned facts, channels and references. It did not request analysis. |
| Explicit reanalysis | Enter on Analyze again requested another attempt; a different snapshot retained the same approved facts and references. |
| Restart | Graceful close and relaunch retained root/location IDs, saved snapshot IDs, facts, references and the Library startup preference. |
| Changed source | Changing only a copied file's modification time and rescanning rejected the old row fingerprint and published a different current snapshot. |
| Missing source | Reversibly renaming the copied FLP to a non-FLP suffix and rescanning reported Missing. Its details contained no facts or references. Restoring it recovered current details. |
| Root controls | Disabling the root through Preferences removed visible current facts. Re-enabling it succeeded. Confirmed removal of the recovery root removed it from Library selection. |
| Invalid FLP | The approved truncated fixture reported fixed `invalid_file` failure with no current facts. Visible Retry increased its durable attempt count. |
| Unavailable parser | Holding only the installed sibling parser produced fixed `parser_unavailable` failure and no references. The three-attempt budget persisted across restart; exhausted Retry remained disabled. |
| Parser recovery | The identical parser bytes were restored. A new modification-time observation of the unchanged fixture bytes allowed fresh analysis; its 137 BPM details returned and persisted through restart. Restart alone did not reset the exhausted budget. |
| Interrupted analysis | The real owned parser child was temporarily suspended so a running lease could be observed. Only its owning test app was force-stopped. Read-only inspection confirmed a durable running job at attempt 2 after exit. Suspension was reversed and the old child retired. Relaunch recovered the job at attempt 3, with identical facts, then another graceful restart retained the result. |
| Accessibility | The installed populated renderer had zero axe WCAG A/AA violations. Keyboard focus and Enter activated refresh/analysis. Emulated widths 900, 560 and 400 had no document horizontal overflow; reduced motion and doubled browser default text size also passed. |
| Inert text and diagnostics | Saved sample references contained no links or actions. The retained application diagnostic log contained none of the selected project, channel, plugin, sample-reference or private run-path values. |

The crash case is controlled fault injection using the actual installed child,
with suspension explicitly reversed before recovery. It does not qualify every
possible Windows crash. The layout widths and text-size change are WebView
emulation; no global Windows display or accessibility setting was changed.

## Approved fixture expectations

The baseline corpus already contains these files; no FLP bytes were invented
or edited for this task. The installed root used renamed copies.

| Existing fixture | Checked facts |
| --- | --- |
| F01 / `FIX-BASE-MIN.flp` | Saved build 24.1.0.4225; supported core facts; unsupported instrument and arrangement fields; empty top-level plugin list without a claim that the project uses no plugins. |
| F06 / `FIX-FL2025-MIN.flp` | Saved build 25.1.3.4922; 130 BPM; inferred default Sampler label/reference, with instrument details still unsupported. |
| F12 / `FIX-FL2026-SAMPLE.flp` | Saved build 26.1.0.5530; 137 BPM; Fixture Sample A; the approved public synthetic sample reference; inferred Sampler and unavailable class/vendor. |
| F13 / `FIX-FL2026-PATTERNS.flp` | End tick 1536; inferred four-bar pattern span and approximately 7.384615 seconds at the saved tempo. This is an arrangement estimate, not rendered audio duration. |
| F14 / `FIX-FL2026-3XOSC.flp` | Two channels in order: Sampler, Fixture Synth A. Plugin positions 1 and 2 retain inferred Sampler and extracted 3x Osc respectively; sample references are unavailable. |
| F04 / `FIX-RB-UNKNOWN.flp` | Partial result preserves supported fields and warns about unverified events. |
| F07 / `FIX-RB-TRUNC.flp` | Fixed invalid-file failure, no current metadata. |

The installed cases include empty references and partial results. Constructed
wrapper/vendor records, duplicate references, long/control-character text,
progressive lists and older response shapes remain covered by PR #251's
unchanged renderer/validator tests. These installed observations do not extend
real FLP compatibility to those constructed examples.

## Screenshots

The desktop images are unmodified component crops from the installed renderer.
The narrow image is an actual viewport capture at an emulated 400-pixel width.
Only approved synthetic saved values appear; the surrounding private root path
is outside these captures. No layout or navigation was hidden for the captures.

- [Installed plugin references, desktop](plugins-desktop.png)
- [Installed sample references, desktop](samples-desktop.png)
- [Installed plugin references, narrow viewport](plugins-narrow-viewport.png)
- [Unavailable-parser failure](parser-unavailable.png)
- [Missing-source details](missing-source.png)

## Binary provenance and retained evidence

| Artifact | SHA-256 |
| --- | --- |
| NSIS installer | `21422eee1594af419f4d2a55b48dbd5b7d7e3a926a8817e5605a1d67924dd57a` |
| Installed desktop | `ee055edda7275950bedc4e99e66ae4baf944244d3505fd914e4c1739c2c7b14f` |
| Matching parser | `179d20a134fd960854318998fbde368402f48f5d4c94132f0a5bdb1287d2f28d` |

Tauri patches the desktop bundle marker during NSIS packaging and restores the
compiler output afterward. The installed executable and retained compiler
output have equal length and differ at exactly three recorded bytes, `UNK`
versus `NSS`. The build log records that packaging operation. The installed
executable is retained independently; its hash above, rather than the
unpatched compiler-output hash, identifies this run. Installed and prepared
parser hashes match exactly.

Raw command results, journal including harness failures/corrections, driver
hashes, databases, logs, fixture hashes and independent binaries are private
under `fruitboard-review-evidence/installed-library-details-20261003`, outside
compiler caches. No raw database, log, FLP or executable is committed here.

All 12 approved source FLPs and eight test copies remained byte-identical.
The test ran under the shared Foundation Smoke host lock. The prior synthetic
profile was reversibly archived, then all 157 original files were restored and
hash-verified after uninstall. Uninstall preserved the tested database, which
is retained with this run. The normal Fruitboard database hash stayed unchanged.
No owned app/parser process or host lock remains.

## Checks and cache handoff

Successful [Foundation CI on the unchanged application source](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/37151170539)
and [CodeQL](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/37151170537)
apply to source `2d5c769`. The pinned enabled installer build and installed
observations above are additional checks. This PR changes documentation and
screenshots only, so another full native rebuild is unnecessary. Local
documentation/privacy/script checks and every final PR CI check are required
before owner review and manual merge.

Codex `/root` owns the reusable temporary cache and coordinated its exclusive
build window and subsequent installed host-lock window. Its absolute location
is recorded in the private handoff. The primary source/cache and all prior
evidence remain protected. Initial storage preflight had 53.79 GiB free and a
4 GiB expected-output allowance; 52.83 GiB remained after uninstall/restore.
The 30 GiB reserve was maintained. Cargo-created aliases were inventoried and
replaced with independent byte-identical cache copies before installed work;
no hardlink cloning or evidence deletion was used.

The existing cache is reusable; retained binaries and reports must remain.
No compiler intermediates were selected for deletion and no new permanent
compiler cache was created. Reuse remains sequential under the named owner.

## Remaining milestone work

This closes the bounded installed integration gap for the current Library
details slices. Metadata remains development-only and this is not the owner's
personal-project testing installer. Plugin Explorer MVP is the next major
slice: grouping and counts from reliable saved references, with explicit
coverage and installation limitations. Resource/performance qualification and
the Phase 3 review checkpoint remain separate gates. Broader parser support,
installed-plugin discovery, audio, sync and later roadmap phases are unchanged.
