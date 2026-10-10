# Saved mixer inserts in project details

Issue [#304](https://github.com/guilhermebmichelin-create/fruitboard/issues/304)
completes the application slice after design PR297, approved F18-F21 corpus
PR299 and parser/full-validation PR301. Baseline is PR303 merge
`719cf51511b1e54b5e606cf1f1745c378e0e0d8b`.

## Meaning and behavior

New analysis stores qualified saved mixer insert count and name states in an
additive `mixerInserts` container within immutable private projection version 1.
Project details show **Saved mixer inserts** after saved patterns. The count
describes explicit saved records, without establishing usage, routing or
audibility. Saved IDs mean positions within this save, not persistent identities
or guaranteed FL Studio track numbers. Duplicate names retain separate entries.
Missing names show fixed explanations; no default labels are invented.

Only FL Studio **26.1.0.5530**'s independently qualified complete eighteen-record
layout is supported: sixteen ordinary positions 1-16, excluding Master/Current
at 0/17. No empty/zero, sparse layout, other builds, effects, routing, nested
plugins, automation or MIDI compatibility is added. Qualified results fit below
the twenty-entry disclosure threshold; constructed component-scale tests cover
disclosure focus without expanding accepted DTO layouts.

Older snapshots lacking the field remain readable and explain that a new
analysis is needed. Refresh reads existing results without queuing analysis or
rewriting snapshots. Missing sections, unadvertised fields, unverified builds/
layouts, ambiguity and exceeded limits retain fixed explanations. Malformed
present data fails the complete read. Loading, errors, changed/missing sources,
disabled roots and discarded late replies never retain an old count as current.

## Storage and authority

The container stores only validated count/name/status/coverage fields. Stored
reads reuse qualified parser validation with fixed ceilings, exact count,
ordered identities, text limits and paired state rules. The renderer validates
the selected DTO, build and identity set; names remain escaped inert text.
Existing source/root/fingerprint/publication/session/lease and details-read
guards remain the authority. No new command, permission, table, schema
migration, snapshot rewrite, file open, audio/MIDI or plugin capability is added.
Rejected publication preserves last-good immutable results. Older readers
ignore the additive field; rollback removes selection/display while retaining
snapshots.

## Validation and resource handoff

Automated checks passed: 131 repository-policy tests, 408 client tests, 536
workspace Rust tests, 118 scanner-enabled desktop tests, 159 analysis-enabled
tests and four supervised native-parser tests. Default/workspace and both
enabled desktop configurations passed warning-denied Clippy. The genuine
mixer cases are compared with the independently registered expectations.
Installed/build evidence will be recorded after those checks complete.
Approved fixture hashes must remain unchanged. Constructed malformed,
older, limit and delayed-transport cases are refusal/UI evidence, not genuine
compatibility. Installed tests use a fresh development identity; personal and
Foundation Smoke profiles stay untouched.

The existing `desktop-feature-tests` validation cache is reused, owner
**Codex /root**; its absolute path is recorded in the private handoff.
Native builds run sequentially after preflight, keeping 30 GiB
free after expected output. Installed tests run alone under the shared host lock
after builds. Private evidence and independently copied binaries remain at
the private `library-mixer-inserts-20261010` evidence packet, outside caches.
No compiler cleanup or new per-PR cache is planned. Final head CI and
owner review/manual merge remain required. No broader Phase 3, resource,
production activation, licensing or distribution acceptance is implied.
