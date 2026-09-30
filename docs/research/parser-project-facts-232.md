# Project dates, saved time and plugin references (issue #232)

The owner requested whole-song arrangement length/duration, plugins used, file
size, last modification, creation date and FL Studio's project-time counter.
Whole song means the maximum arrangement clip end, not a sum of clip lengths
or each individual Playlist track. This change extends the native parser and
its validated projection; it does not activate parsing in the scanner or UI.

## Project information format and evidence

[PyFLP's published project format reference](https://pyflp.readthedocs.io/en/latest/_modules/pyflp/project.html)
describes event 237 as two little-endian 64-bit floating-point values: creation
as days from 1899-12-30 in local wall time, and time spent as elapsed days.
The independent Rust decoder implements these format facts without copying
PyFLP code or adding a dependency. It requires exactly 16 payload bytes,
checks finite values and bounded ranges, rounds to milliseconds and rejects
ambiguous repeated records. Creation dates are restricted to 1900 through 9999. The saved date carries `timezone: unspecified-local`; no UTC offset is
invented. The time counter is FL Studio's saved observation, not an independent
measurement of productive activity.

Read-only inspection of the unchanged approved corpus found the record in the
maintained 2024, 2025 and 2026 saves. All valid approved fixtures pass the new
projection. For example, the F11/F13/F14 creation serial is
`46280.85352434028`; their saved elapsed values differ. These are observations
of existing approved bytes, not newly registered GUI ground truth or proof of
the counter's idle/reset behavior. Tests separately construct known calendar
and elapsed-time values, including 1900 and 2000 leap-year boundaries,
non-finite/range errors, missing and duplicate records. Broader matching-build
GUI qualification remains necessary before expanding compatibility claims.

Filesystem creation time comes from the same authorized open handle as the
read. It is optional, uses a distinct field and is never substituted for the
embedded creation date. Size and modification time in the typed result come
from the native request after its returned fingerprint and current observations
have been checked. Existing Library size/modified facts are unchanged.

## Saved plugin references

[The published plugin format reference](https://pyflp.readthedocs.io/en/latest/_modules/pyflp/plugin.html)
identifies event 201 as the stored class name and event 213 as plugin data.
The new collection retains inert top-level names from all such class records,
including effect names outside channel contexts. An editable channel label
cannot replace a class name. Empty slots are skipped; the verified empty
Sampler class in known channel contexts keeps its explicit default inference.
Unknown or malformed class encodings retain a gap rather than guessing.

For `Fruity Wrapper` only, recognized type markers 8 and 10 permit a bounded
walk of tagged records with a 32-bit tag and 64-bit payload length. Tags 54
and 56 contain UTF-8 factory name and vendor. This walks lengths over other
payloads without copying them. Plugin paths, preset state, IDs and DLL data
are neither returned nor loaded. Duplicate records, unknown markers, malformed
lengths/UTF-8/NUL and overlong text return an unsupported name/vendor.
The collection is capped at 1024 references and wrapper walks at 1024 records;
the existing process response cap still applies.

The F14 approved GUI fixture verifies Sampler and `3x Osc`. Constructed
in-memory cases cover other saved class labels, VST name/vendor framing,
malformed data and private path/state exclusion. They validate decoding and
boundaries, not independent real-world VST compatibility. The output therefore
states `coverage: top-level-saved-references`. It does not claim a complete
nested Patcher inventory, installation/availability, VST2/VST3 classification,
role or canonical identity. Repeated references remain separate observations.

## Arrangement length and validated projection

The existing verified pattern placements provide the greatest clip end in
ticks. The new bars estimate divides that end by 384 on the verified 96-PPQ,
4/4 layout. Nominal seconds continue to use the saved base BPM. Both estimates
carry low confidence and named methods. Missing/unknown clips or timing return
unavailable/unsupported. Audio clips, tempo automation, multiple arrangements
and rendered sound tails remain outside this verified span.

`validate_project_reply` requires a descriptor advertising the added fields and
returns private typed project metadata. It first applies the initial field,
fingerprint and native freshness validator, then validates local calendar dates,
integer time values, saved plugin reference bounds and known provenance. It
recomputes the maximum clip end and checks the bars/seconds formulas and their
absence/unsupported relationships. Unknown unrelated JSON is discarded.
The original initial-only validator remains available. Neither result type has
automatic metadata Debug/Serialize implementations. Native integration must
still recheck freshness in the eventual publication transaction.

Parser regression and real-supervisor tests preserve all twelve approved FLP
files byte-for-byte. No new fixture, dependency, plugin load, filesystem scope,
database migration, renderer command, packaging or phase acceptance is added.
