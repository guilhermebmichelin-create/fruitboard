# Public FLP version-comparison research

Issue: [#399](https://github.com/guilhermebmichelin-create/fruitboard/issues/399).
Reviewed 2026-11-04 for the [cross-build pilot](cross-build-pilot-399.md).
Status: **Literature findings and test hypotheses, not additional Fruitboard
compatibility qualification. No external parser or fixture was installed.**

## Owner summary

Yes, other projects have compared FLP versions. Their findings give us concrete
changes to investigate and useful candidate releases, even before another
matching FL Studio program is available. A literature survey cannot establish
which additional builds run on this machine or validate Fruitboard's output.
The existing four-control baseline remains the only new executed result here.

## Sources and useful findings

| Primary source                                                                                                                                                                                                                        | Findings reported by its authors                                                                                                                                       | Use for Fruitboard                                                                                                                                              |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [flpdiff FL25 format report](https://github.com/dawhubapp/flpdiff/blob/28aae8f66471be9bf83e342089bbf5b627cfe309/docs/fl-format/fl25-event-format.md)                                                                                  | Observes FL25.2.4.4960 on macOS. Event `0xAC`/172 has three payload bytes; reading four consumes the following event marker and misaligns the stream.                  | Prioritize exact-build event framing, then compare selected basic fields. This is a reported observation, not proof of the first release containing the change. |
| [flpdiff compatibility overview](https://github.com/dawhubapp/flpdiff/blob/28aae8f66471be9bf83e342089bbf5b627cfe309/README.md#fl-studio-compatibility)                                                                                | Reports testing 85 projects across FL9-25.                                                                                                                             | A useful wider survey; its parser success percentages do not establish independently checked correctness for every field/build.                                 |
| [PyFLP format documentation](https://pyflp.readthedocs.io/en/stable/architecture/flp-format.html) and [version-dependent decoding](https://github.com/demberto/PyFLP/blob/f937126b888ce94271bfea631b89166c74056530/pyflp/__init__.py) | Documents the event-stream structure; source switches text decoding at FL11.5.                                                                                         | Historical encoding boundary; not a modern-build compatibility certificate.                                                                                     |
| [PyFLP Playlist source](https://github.com/demberto/PyFLP/blob/f937126b888ce94271bfea631b89166c74056530/pyflp/arrangement.py)                                                                                                         | Handles 32/60-byte records and marks the additional tail as new in FL21.                                                                                               | Another field-specific transition to investigate, rather than treating all fields as one format version.                                                        |
| [flpkit reader and verification notes](https://github.com/clarkipeng/flpkit/blob/5f8fc4f9824b618a3437b4ecd0e985cca171b8c9/README.md)                                                                                                  | Reports Playlist record sizes of 32/60/80/88 bytes, a one-byte event-172 override for FL2026, differential reading across 164 projects and selected live macOS checks. | Investigate layout detection and exact-build differences. Its automation observations are also a lead for #324, not a qualified Fruitboard grammar.             |
| [SpendLess 1.3.3 changelog](https://spendlessdaw.app/changelog)                                                                                                                                                                       | Reports fixing FL25/26 stream alignment, replacing heuristic event sizing and adding FL26 preamble regressions in its bundled ts-flp parser.                           | Independent evidence that modern framing deserves targeted tests; its private corpus is not independently reproduced here.                                      |

Repository sources are pinned to the inspected revisions. PyFLP's inspected
revision dates to 2023; modern third-party claims need their own evidence review.
No implementation was copied or dependency selected by this survey.

## What needs resolution

The flpdiff report describes three-byte event172 in its exact FL25/macOS case;
flpkit describes a one-byte FL2026 override without an exact build in that
README statement. These are different claimed scopes. They do not establish a
contradiction on the same bytes, nor a universal width for FL25 or FL26.

The [flpdiff correction discussion](https://github.com/dawhubapp/flpdiff/issues/1)
also records an earlier mistaken event interpretation. Fruitboard should test
event boundaries against an exact genuine file and independent expectations,
not choose a width merely because a parser reaches the end of a file.
Fruitboard's existing qualified three-byte handling stays unchanged.

## Official release information

[Image-Line's changelog](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/WhatsNew.htm)
lists FL25.2.4 on February 5, 2026 and FL26.1.1 on July 9, 2026. It also records
a project-file identity change in FL26 beta9: storing a license plate instead
of a username. This is useful for planning renewed privacy inspection of future
fixture candidates. Release notes describe selected behavior changes; they are
not a complete binary format specification.

[Image-Line's older-installer instructions](https://support.image-line.com/action/knowledgebase?ans=300)
point owners to the authenticated customer archive. The
[FL25.2.4 RC announcement](https://forum.image-line.com/viewtopic.php?t=339034)
lists different Windows/macOS build suffixes. Therefore the literature's
macOS build4960 must not be assumed to identify a Windows installer.
No installer was downloaded and no installed build availability was inferred.

## Updated next work

1. Prioritize the **25.2.4 release line** for the reported framing transition,
   and an adjacent **26.1.x release** for a neighboring-build comparison.
   These are candidate families, not registered exact-build experiments.
2. Continue read-only format/provenance analysis without waiting for the owner
   to remember build numbers. Use official release/archive information to
   identify an exact available platform/build before authoring new saves.
3. Register independent GUI expectations, exact bytes and per-field hypotheses
   before extraction. Resolve event framing first, followed by the pilot's
   version/tempo/channels/sample-text fields. Playlist, automation and mixer
   interpretation remain separate groups.
4. Treat upstream corpora as leads. Establish provenance, redistribution terms,
   privacy and independent expected values before using any outside file as
   Fruitboard evidence. No external FLP is added by this documentation update.
5. Keep the production allowlist, limits, unsupported states and current report
   unchanged until genuine-file checks justify a separately reviewed expansion.

Owned changes are this literature note and the pilot pointer only. Scoped
Markdown/format/privacy/whitespace checks and normal changed-head CI are required.
No local build/cache or installed/qualification window is needed. Existing
baseline/runtime evidence remains tied to its original source. Owner `/root`
preserves the existing caches and evidence; absolute paths, capacity and this
survey's source pins remain in the private cross-build-pilot-20261104 handoff.
