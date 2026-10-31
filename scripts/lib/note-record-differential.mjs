// Research only: positional byte comparisons do not qualify musical fields.
import { createHash } from "node:crypto";
import { inspectNoteRecordCandidates } from "./note-record-research.mjs";

const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const WORDS = [
  "positionWord",
  "flagsWord",
  "channelWord",
  "lengthWord",
  "keyWord",
  "byte21",
];

export function compareNoteRecordCandidates(baselineBytes, candidateBytes) {
  const baseline = inspectNoteRecordCandidates(baselineBytes);
  const candidate = inspectNoteRecordCandidates(candidateBytes);
  if (
    baseline.ppqHeader !== candidate.ppqHeader ||
    baseline.patterns.length !== candidate.patterns.length ||
    baseline.patterns.some(
      (pattern, index) =>
        pattern.patternId !== candidate.patterns[index].patternId ||
        pattern.records.length !== candidate.patterns[index].records.length,
    )
  )
    throw new Error("RESEARCH_COMPARISON_LAYOUT");
  if (baseline.recordCount === 0)
    throw new Error("RESEARCH_COMPARISON_NO_RECORDS");

  return {
    researchOnly: true,
    musicalFieldsQualified: false,
    alignment: "Pattern ID and source ordinal only; not verified note identity",
    scope: "Note payload bytes only; other FLP content is not compared",
    savedBuild: baseline.savedBuild,
    ppqHeader: baseline.ppqHeader,
    baselineSha256: hash(baselineBytes),
    candidateSha256: hash(candidateBytes),
    recordCount: baseline.recordCount,
    patterns: baseline.patterns.map((pattern, index) => ({
      patternId: pattern.patternId,
      records: pattern.records.map((record, ordinal) => {
        const other = candidate.patterns[index].records[ordinal];
        return {
          ordinal,
          baselineFileOffset: record.fileOffset,
          candidateFileOffset: other.fileOffset,
          changedBytes: record.recordBytes.flatMap((value, offset) =>
            value === other.recordBytes[offset]
              ? []
              : [
                  {
                    offset,
                    baseline: value,
                    candidate: other.recordBytes[offset],
                  },
                ],
          ),
          changedCandidateWords: WORDS.filter(
            (field) => record[field] !== other[field],
          ).map((field) => ({
            field,
            baseline: record[field],
            candidate: other[field],
          })),
        };
      }),
    })),
  };
}
