#!/usr/bin/env node
// Fixed approved fixtures only. No arbitrary path, FLP writing or product wiring.
import { createHash } from "node:crypto";
import { closeSync, fstatSync, openSync, readSync } from "node:fs";
import { inspectNoteRecordCandidates } from "./lib/note-record-research.mjs";

const fixtures = [
  {
    id: "F16",
    name: "FIX-FL2026-NOTES.flp",
    bytes: 48431,
    sha256: "be58afee914e10ab95f73f55405d8ea72f607430016a4264ecd1d5a488ef041f",
  },
  {
    id: "F17",
    name: "FIX-FL2026-NAMED-EMPTY.flp",
    bytes: 46697,
    sha256: "32cee5af3969d61c54f6bb6703fc1f3406be509d33a81e57b571d1c83cf7f78b",
  },
];
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
function readApproved(fixture) {
  const path = new URL(
    `../fixtures/parser-corpus/${fixture.name}`,
    import.meta.url,
  );
  const fd = openSync(path, "r");
  try {
    if (!fstatSync(fd).isFile() || fstatSync(fd).size !== fixture.bytes)
      throw new Error("RESEARCH_APPROVED_SIZE_MISMATCH");
    const bytes = Buffer.alloc(fixture.bytes + 1);
    let used = 0;
    while (used < bytes.length) {
      const count = readSync(fd, bytes, used, bytes.length - used, used);
      if (count === 0) break;
      used += count;
    }
    if (used !== fixture.bytes || fstatSync(fd).size !== fixture.bytes)
      throw new Error("RESEARCH_INPUT_CHANGED");
    return bytes.subarray(0, used);
  } finally {
    closeSync(fd);
  }
}
try {
  if (process.argv.length !== 2) throw new Error("RESEARCH_NO_ARGUMENTS");
  const results = fixtures.map((fixture) => {
    const before = readApproved(fixture);
    if (hash(before) !== fixture.sha256)
      throw new Error("RESEARCH_APPROVED_HASH_MISMATCH");
    const observation = inspectNoteRecordCandidates(before);
    if (hash(readApproved(fixture)) !== fixture.sha256)
      throw new Error("RESEARCH_INPUT_CHANGED");
    return { fixture: fixture.id, sha256: fixture.sha256, ...observation };
  });
  console.log(JSON.stringify({ researchOnly: true, results }, null, 2));
} catch {
  // Do not print arbitrary filesystem errors or private paths.
  console.error("Approved note-record research failed; no report produced.");
  process.exitCode = 1;
}
