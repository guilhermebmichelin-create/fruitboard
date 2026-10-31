#!/usr/bin/env node
// Explicit hash-pinned synthetic inputs only. Never enumerate project roots.
import { createHash } from "node:crypto";
import { closeSync, fstatSync, openSync, readSync } from "node:fs";
import { compareNoteRecordCandidates } from "./lib/note-record-differential.mjs";

const MAX_FILE_BYTES = 64 * 1024 * 1024;
function readPinned(path, expectedHash) {
  const fd = openSync(path, "r");
  try {
    const stat = fstatSync(fd);
    if (!stat.isFile() || stat.size > MAX_FILE_BYTES || stat.size < 22)
      throw new Error("RESEARCH_INPUT_SIZE");
    const bytes = Buffer.alloc(stat.size + 1);
    let used = 0;
    while (used < bytes.length) {
      const count = readSync(fd, bytes, used, bytes.length - used, used);
      if (count === 0) break;
      used += count;
    }
    if (used !== stat.size || fstatSync(fd).size !== stat.size)
      throw new Error("RESEARCH_INPUT_CHANGED");
    const result = bytes.subarray(0, used);
    if (createHash("sha256").update(result).digest("hex") !== expectedHash)
      throw new Error("RESEARCH_INPUT_HASH");
    return result;
  } finally {
    closeSync(fd);
  }
}

try {
  const [baselinePath, baselineHash, candidatePath, candidateHash] =
    process.argv.slice(2);
  if (
    process.argv.length !== 6 ||
    !/^[a-f0-9]{64}$/.test(baselineHash ?? "") ||
    !/^[a-f0-9]{64}$/.test(candidateHash ?? "")
  )
    throw new Error("RESEARCH_ARGUMENTS");
  const result = compareNoteRecordCandidates(
    readPinned(baselinePath, baselineHash),
    readPinned(candidatePath, candidateHash),
  );
  // Reopen and recheck before publishing a report. This is not a filesystem
  // authority boundary; use independent, quiescent synthetic experiment files.
  readPinned(baselinePath, baselineHash);
  readPinned(candidatePath, candidateHash);
  console.log(JSON.stringify(result, null, 2));
} catch {
  // Filesystem errors, arbitrary arguments and source text stay private.
  console.error("Note-record comparison failed; no report produced.");
  process.exitCode = 1;
}
