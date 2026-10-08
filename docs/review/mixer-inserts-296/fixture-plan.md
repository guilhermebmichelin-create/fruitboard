# Genuine mixer fixture gate

Status: **Completed for exact F18-F21 bytes approved on 2026-10-08.**
The [qualification result](../../research/parser-mixer-inserts-298-result.md)
records GUI observations, the eighteen-record layout, positional identity,
sixteen ordinary records and remaining unqualified cases. The preregistered
procedure below remains the historical creation/qualification boundary.
Scope and planning boundary are in the [design](README.md). Candidate exact
saved build: **26.1.0.5530**. The fifteen previously approved corpus files remain
unchanged; four separately approved mixer fixtures are added by issue #298.

## Record expectations before byte inspection

Use a private copy of the approved minimal empty-Sampler baseline, with no
personal music, notes, audio, external plugins or altered routing. Record the
matching FL Studio executable version/hash, baseline fixture hash, intended GUI
changes, selected positions and exact labels before creating candidates. Confirm
the actual build; a mismatch stops qualification instead of expanding support.
Do not inspect original or candidate mixer bytes until GUI expectations are
registered. Parser output and agreement between decoders are not ground truth.

| Candidate slot | Controlled GUI case | Independently recorded expectations |
| --- | --- | --- |
| F18, subject to allocation | Save with mixer untouched | Names/defaults and special tracks visible after reopen; do not prespecify a saved total or assume absence/zero |
| F19, subject to allocation | Rename ordinary GUI inserts 2 and 7 to `Fixture Mixer A` and `Fixture Mixer B`; give the empty Channel Rack and pattern distinct `Fixture Channel Context` / `Fixture Pattern Context` names | Selected GUI positions and exact reopened labels persist; unchanged default/special-track controls; these positions are not claimed saved IDs or a total of two |
| F20, only if needed for identity qualification | On a separate baseline copy, give the same two ordinary GUI inserts the identical label `Fixture Mixer Same` | Two distinct GUI identities survive with the same label; names cannot supply identity or deduplicate records |

These slots are proposals; inspect actual manifest allocation before use.
No notes, playback, effect additions or routing changes are needed. Capture
all observations privately, not personal paths/registration details in public
reports. Any case requiring a broader GUI change needs a revised preregistration
before saving. Reopen and inspect without saving/playing; hash before/after.

The GUI establishes intended labels and distinguishes ordinary inserts from
Master/Current. It does **not** directly establish a saved-record count. After
GUI registration, use independent bounded read-only byte observations and
controlled file comparisons to establish which records are explicitly stored,
their identity representation, framing, name encoding and section boundaries.
Keep GUI and structural observations separately labeled. Publish counts only
when both observations qualify the proposed meaning, including unnamed records.

## Required format and compatibility decision

Record actual event/aggregate framing, entry/exit/unknown transitions, identity
scope and range, optional name representation, duplicate handling, complete
section termination and the independently verified special-track distinction.
Compare renamed saves against the untouched save: unrelated channel/pattern
labels must not bind to mixer records. Record the ordering actually observed;
do not extrapolate to additional layouts or builds. If the minimal files cannot
establish a necessary boundary, preregister a separate minimal case or leave
the affected field unsupported. Constructed transitions can test defenses but
cannot qualify unseen genuine layouts.

Determine whether untouched defaults are saved explicitly or omitted. Do not
equate GUI emptiness, a missing name, the number of custom labels or the highest
ID with a saved total. Do not assign default names or Master/Current numeric IDs
from assumptions. An explicit zero encoding remains unsupported unless a
separate genuine case can independently qualify it. Name omission is not empty
text; qualify any explicit empty-name representation separately before support.

If identity/framing/special-track exclusion cannot be qualified, stop extraction
and report the specific missing evidence. If counts/names require a different
contract, review the amended design before implementation. A successful save
or a parser match alone cannot pass this gate. No unofficial implementation is
copied, translated or used as the authority for expectations.

## Privacy, exact approval and corpus slice

Keep all new FLPs private until review. Independently inventory and sanitize
only narrow registration/project-identifying data, without changing mixer
records. Retain original/sanitized hashes and changed-byte inventory privately.
Reopen the exact sanitized candidate read-only and verify GUI expectations again;
compare structural observations against the original and prove unchanged mixer
semantics. If safe sanitation is uncertain, keep the file private and report it.

Prepare `FIXTURE_APPROVAL.md` with exact candidate paths, sizes, SHA-256 values,
expectation-file hash, provenance/license, privacy review and intended public
scope. Ask the owner about those exact bytes/expectations only after that
reviewable packet exists. Approval of F15, F16/F17 or this design does not cover
these new candidates. No broad search or personal project-root enumeration.

After exact approval, publish only approved bytes/expectations in a focused
corpus PR: force-add named FLPs, update manifest/privacy allowlist and recorded
research/qualification result. Expected mixer values must come from the
registered GUI plus independent structural observations, never product output.
Record build/layout support and each unqualified case explicitly. Hash every
approved corpus FLP before and after checks. Parsing comes in a later PR.

## Validation and host/storage boundary

The [implementation plan](implementation-plan.md) names owned paths, accepted
source boundaries, single cache owner, ≥30 GiB reserve and evidence handoff.
The fixture task records its actual clean worktree and private candidate/evidence
paths before starting. Genuine GUI qualification runs alone, separately from
heavy builds/tests. Do not open/save the owner's projects or profiles, kill
unrelated FL Studio sessions, or alter the installed development app.

Creating candidates needs a storage preflight charged for expected fixture and
evidence output. No local native build is required for a corpus-only slice;
run pinned privacy/Markdown/script/policy checks and normal updated-head CI.
Preceding successful CI applies only to unchanged code, not new mixer ground
truth. Preserve private originals, approval and structural evidence outside
compiler caches; no cleanup of owner data or earlier evidence is authorized.
