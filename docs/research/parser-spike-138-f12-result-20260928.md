# Parser spike #138: F12 positive sample-reference follow-up

- Date: 2026-09-28, after the approved F12 fixture merged in PR #170.
- Question: can the existing bounded Rust research parser extract an explicit
  channel name and raw sample reference from the FL Studio 2026 F12 save, and
  does the pinned PyFLP comparison change the parser recommendation?
- Scope: the ten approved files in the
  [manifest](../../fixtures/parser-corpus/manifest.md). F02 and F03 remain
  owner-recorded `not covered`. The manifest's expected fields and hashes were
  registered before any parser read F12.
- Result: Rust matched all four registered F12 fields. The original F06/F11
  default-channel-name mismatches remain. No production parser is selected.

## Method and inputs

The F12 input was the exact owner-approved, 46,703-byte sanitized file,
`FIX-FL2026-SAMPLE.flp`, SHA-256
`dc11a613e34f2ec4918addf1ec2562d6a2c75ebc8c322f56bcb42d280e607b39`.
The matching FL Studio 2026 build had already reopened it without saving and
showed 137 BPM, Sampler `Fixture Sample A`, and `fixture-silence.wav` loaded.
The generated silent WAV remained outside Git. This run used no personal
project, plugin, or new fixture bytes.

The research-only Rust parser source was unchanged from commit `17a5114`; its
`main.rs`, `Cargo.toml`, and `Cargo.lock` Git blob IDs match this follow-up's
source tree. The independently retained optimized binary was 258,048 bytes,
SHA-256
`527be990796574a81064753e7af2b2d80a5349a26fad7da20e3a44abb93d111f`.
No local build was needed. Harness commit `b314977` added F12's exact hash,
expected values, and a regression test that rejects an absent sample reference.
The bounded runner applied a 10-second per-process timeout, 256 KiB stdout and
64 KiB stderr limits, and fixture-hash checks before and after each parse.
Nine local Python harness tests passed. Reports are retained outside both Git
and the compiler cache.

The stable PyFLP baseline used the same isolated Python 3.11.16 / PyFLP 2.2.1
environment as the original result. The optional enum compatibility diagnostic
was run separately; it changes only the probe child process, not the installed
package or product. Neither PyFLP run provides a safety or performance
qualification.

## Observed result

| F12 registered field | Rust observation | PyFLP 2.2.1 baseline | PyFLP enum diagnostic |
| --- | --- | --- | --- |
| Saved version | `26.1.0.5530` extracted | No fields: `TypeError` | `26.1.0.5530` extracted |
| Base tempo | 137 BPM extracted | No fields | Unavailable (`PYFLP_RETURNED_NONE`) |
| Channel names | `["Fixture Sample A"]` extracted | No fields | Failed (`NoModelsFound`) |
| Raw sample references | Exact Public Documents F12 `fixture-silence.wav` path extracted | No fields | Failed (`NoModelsFound`) |
| Overall | `complete`, no diagnostics; all registered checks passed | Failed | Parsed container, incomplete fields |

Rust returned the exact raw path registered in the manifest, including its
Windows separators:
`C:\Users\Public\Documents\FruitboardFixtures\F12\fixture-silence.wav`.
Its F12 read/parse observation was 160 microseconds, with a 4,079,616-byte peak
working set; the median of its warm process launches was about 6.02 ms. These
are observations on one host, not a cold-start or performance qualification.

All ten Rust runs preserved input hashes. The only registered-field mismatches
remained F06 and F11 `channelNames`: those FL Studio 2025/2026 files display the
built-in default `Sampler` but do not store that name as an explicit event.
The other registered fields and robustness outcomes retained their original
results. Stable PyFLP still produced no complete parse on any of the ten
fixtures (nine `TypeError` results and one truncated-file `HeaderCorrupted`).
With the optional enum diagnostic, F12 exposed only its version. The diagnostic
still did not meet the matrix or the F10 channel-count safety requirement.

The private bounded reports have these SHA-256 values:

| Report | SHA-256 |
| --- | --- |
| Rust, ten fixtures | `094f291c93f58f3ec45baddc0962c7afae87e1b67e43ba607dd9abf83c416546` |
| Stable PyFLP baseline, ten fixtures | `bbf201fe6b417e5e39f2fab24de1c8fbf9b1007f89fe97f40e297f6d342cdf7e` |
| Opt-in PyFLP enum diagnostic, ten fixtures | `3c0b169e84b9d27d75eaa9b4d78e9524cdc5053e312bd7a4addc0a77f0244c` |

## Decision input and limits

F12 establishes one positive raw sample-reference extraction and one explicit
channel name in a genuine FL Studio 2026 save. It does not establish those
features across all supported versions, infer unstored default channel names,
or cover absent tempo, zero channels, FL Studio 20/21, Unicode names, or hostile
untrusted projects. The Rust prototype still misses the registered F06/F11
channel-name values, so the four-field acceptance matrix remains unmet. PyFLP
is still unsuitable as the tested fallback and remains blocked from product
distribution by the existing license decision.

The recommendation remains to **defer production parser selection** until the
owner decides how the adapter should represent a channel name that FL Studio
displays but does not store. A separate, recorded owner decision is required
before choosing Rust, PyFLP, stopping, or implementing the Phase 3 production
parser. Neither the pre-registered fixture expectations nor ADR-002 were changed
to make this research result pass.
