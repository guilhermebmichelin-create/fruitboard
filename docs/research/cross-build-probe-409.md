# Supervised shared-decoder comparisons

Issue: [#409](https://github.com/guilhermebmichelin-create/fruitboard/issues/409).
Parent: [cross-build pilot #399](cross-build-pilot-399.md) and
[shared-decoder probe #401](shared-decoder-research-401.md).
Source boundary: merged `eaa0f978bf5e12ab6deaa7bd743df2739e9538e9` (PR402).
Status: **Fixed-control comparison tooling; additional builds remain not tested.**

## What changed

The existing probe emits observations under three explicit event172-width
hypotheses. The new supervisor compares every pilot field against the registered
independent expectations, preserving extracted, inferred and unavailable states.
A complete decoder outcome cannot conceal a mismatching value. Policy refusals,
unsupported layouts and failures stay distinct; a failure is not automatically
evidence of format incompatibility.

```text
node scripts/research-cross-build-probe.mjs --probe <retained-probe.exe> --sha256 <expected-lowercase-sha256>
```

The runner accepts only a hash-pinned executable, runs it with no arguments,
and bounds execution to ten seconds, stdout to 256 KiB and accepted stderr to
64 KiB. The existing probe still reads only F01/F06/F11/F12. The supervisor
independently checks those fixed inputs and the executable before and after the
child, including lengths, hashes, identity and modification time. It requires
all four control identities/hashes and all three registered profiles; incomplete
or malformed output produces only a fixed diagnostic and no comparison report.

Reports contain known expectations, allowlisted classifications and bounded
numeric framing summaries. Arbitrary process text, returned field values,
decoder reasons and extra properties are not echoed. Framing prefixes must be
contiguous, bounded to 32 records and marked truncated when appropriate.

For every pair of hypotheses whose five fields match, the report records whether
the captured boundaries or total event counts differ. An indistinguishable pair
means only that the captured evidence cannot distinguish it; the prefix does
not establish identical whole-file framing. No width is selected, even when only
one hypothesis matches. Reports remain explicitly unqualified and do not have
an overall compatibility pass flag. The command can complete successfully while
reporting mismatched or failed experimental hypotheses.

## Executed evidence

The [retained result](cross-build-probe-results-409.json) records twelve
hypotheses and sixty field comparisons. Ten hypotheses match all five registered
fields. F11/F12 match under both one- and three-byte framing while their event
counts differ (1,423 versus 1,421); four-byte framing fails its five fields.
F01/F06 match all three hypotheses, indistinguishable in captured framing
evidence. These observations reproduce the previous probe findings through an
automated comparison rather than choosing a successful interpretation.

The independently retained research executable SHA-256 is
`421812f3ac40633fe4cd4f5d7a6b197e3049c6dd5b858bc25a9e6d9d37323907`, built at
`7d987c30af3f8ade29e3ee24dd6e475188fdd797`. Its compiled parser sources,
Cargo manifest/lock, toolchain and embedded control specification match PR402's
merged baseline; only its parser README differs. The prior build receipt and
independent binary remain private. No native rebuild was needed. The separate
production runner also repeats the four-control/twenty-field baseline using
the previously retained production parser; this is unchanged-build evidence.

On this host, executable version resources identify only the already represented
24.1.0.4225, 25.1.3.4922 and 26.1.0.5530 builds. FL Studio 20/21 directory names
alone do not identify runnable programs. No candidate was launched, downloaded,
registered from an About screen or used to create a save. The proposed adjacent
26.1.x and transition 25.2.4 candidates still need exact available programs,
independent GUI expectations and approved genuine-save bytes. This report
adds neither a supported build nor a new fixture.

## Validation and storage handoff

Owned source: primary checkout's Node supervisor, comparison helper, focused
tests and research/status documentation. No parser/native/client, fixture,
dependency or CI command changes. Required local checks: focused and all root
Node tests, script syntax, scoped formatting/Markdown, privacy and whitespace.
Normal updated-head CI remains required; PR402 evidence applies only to
unchanged compiled inputs. No installed or performance qualification is claimed.
Local checks pass: 215 root Node tests, one expected platform skip, script
syntax, scoped formatting/Markdown, repository privacy and whitespace.

No local build/cache write or heavyweight/installed host window is needed.
Owner `/root` preserves the primary `target` and owns sequential reuse of the
existing `desktop-feature-tests` validation cache; absolute paths are recorded
in the private handoff.
C: measured about 20.9 GiB free, below the required 30 GiB reserve; builds and
fixture generation are blocked pending capacity restoration and a fresh
four-path preflight. Lightweight source checks continue with under 0.01 GiB
additional receipts. Private evidence is retained outside compiler caches in
`cross-build-supervision-20261111` (absolute location in the private handoff).
Preserve existing binaries, fixtures and owner planning drafts. No deletion,
new compiler cache or cleanup-related rebuild cost is introduced.
