#!/usr/bin/env node
// Fixed approved controls through the unchanged production protocol. No bypass.
import { spawnSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import {
  closeSync,
  fstatSync,
  lstatSync,
  openSync,
  readFileSync,
  readSync,
  realpathSync,
  statSync,
} from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  compareResearchFields,
  readResearchReply,
} from "./lib/cross-build-research.mjs";

const root = realpathSync(
  fileURLToPath(new URL("../fixtures/parser-corpus", import.meta.url)),
);
const specification = JSON.parse(
  readFileSync(
    new URL("../research/cross-build-controls.json", import.meta.url),
  ),
);
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");

function readControl(control) {
  const path = join(root, control.file);
  if (lstatSync(path).isSymbolicLink() || realpathSync(path) !== path)
    throw new Error("RESEARCH_INPUT_PATH");
  const fd = openSync(path, "r");
  try {
    const before = fstatSync(fd);
    if (!before.isFile() || before.size !== control.bytes)
      throw new Error("RESEARCH_INPUT_SIZE");
    const buffer = Buffer.alloc(control.bytes + 1);
    let size = 0;
    while (size < buffer.length) {
      const count = readSync(fd, buffer, size, buffer.length - size, size);
      if (!count) break;
      size += count;
    }
    const after = fstatSync(fd);
    if (
      size !== control.bytes ||
      after.size !== before.size ||
      after.mtimeMs !== before.mtimeMs ||
      hash(buffer.subarray(0, size)) !== control.sha256
    )
      throw new Error("RESEARCH_INPUT_CHANGED");
    return { path, size, modified: before.mtimeMs, inode: before.ino };
  } finally {
    closeSync(fd);
  }
}

function parserHash(parser) {
  const stat = statSync(parser);
  if (!stat.isFile() || stat.size > 128 * 1024 * 1024)
    throw new Error("RESEARCH_PARSER_SIZE");
  return hash(readFileSync(parser));
}

try {
  const args = process.argv.slice(2);
  if (
    args.length !== 4 ||
    args[0] !== "--parser" ||
    args[2] !== "--sha256" ||
    !/^[a-f0-9]{64}$/.test(args[3])
  )
    throw new Error("RESEARCH_ARGUMENTS");
  const parser = realpathSync(args[1]);
  if (parserHash(parser) !== args[3]) throw new Error("RESEARCH_PARSER_HASH");
  // Inspect all registered inputs before launching any child process.
  const inputs = specification.controls.map(readControl);
  const controls = specification.controls.map((control, index) => {
    const before = inputs[index];
    const id = randomUUID();
    const request = {
      protocolVersion: 1,
      schemaVersion: 2,
      id,
      method: "parse",
      params: {
        path: before.path,
        expected: {
          size: before.size,
          modifiedAtMs: Math.trunc(before.modified),
        },
        allowedRoots: [root],
        features: ["basic-metadata"],
      },
    };
    const child = spawnSync(parser, [], {
      input: JSON.stringify(request) + "\n",
      encoding: "utf8",
      timeout: 10000,
      maxBuffer: 256 * 1024,
      windowsHide: true,
      shell: false,
    });
    const after = readControl(control);
    if (
      after.modified !== before.modified ||
      after.inode !== before.inode ||
      child.error ||
      child.status !== 0 ||
      child.stderr.length > 64 * 1024
    )
      throw new Error("RESEARCH_PROCESS_OR_INPUT_FAILED");
    const result = readResearchReply(child.stdout, id);
    const fingerprint = result.inputFingerprint;
    if (
      fingerprint?.size !== before.size ||
      fingerprint?.modifiedAtMs !== Math.trunc(before.modified) ||
      fingerprint?.hash?.algorithm !== "sha256" ||
      fingerprint?.hash?.value !== control.sha256
    )
      throw new Error("RESEARCH_RESULT_FINGERPRINT");
    const fields = compareResearchFields(control.fields, result);
    return {
      id: control.id,
      sha256: control.sha256,
      passed:
        result.outcome === "complete" &&
        fields.every((f) => f.classification.startsWith("matched_")),
      fields,
    };
  });
  // Recheck every file after all children, including earlier controls.
  specification.controls.forEach((control, index) => {
    const current = readControl(control);
    if (
      current.inode !== inputs[index].inode ||
      current.modified !== inputs[index].modified
    )
      throw new Error("RESEARCH_INPUT_CHANGED");
  });
  if (parserHash(parser) !== args[3])
    throw new Error("RESEARCH_PARSER_CHANGED");
  const passed = controls.every((control) => control.passed);
  console.log(
    JSON.stringify(
      {
        researchOnly: true,
        productionPolicyBypassed: false,
        parserSha256: args[3],
        runnerSha256: hash(readFileSync(fileURLToPath(import.meta.url))),
        comparisonSha256: hash(
          readFileSync(
            new URL("./lib/cross-build-research.mjs", import.meta.url),
          ),
        ),
        specificationSha256: hash(
          readFileSync(
            new URL("../research/cross-build-controls.json", import.meta.url),
          ),
        ),
        sourceHashesUnchanged: true,
        passed,
        controls,
        additionalBuilds: [
          { role: "adjacent", state: "not_tested", exactBuild: null },
          { role: "format_transition", state: "not_tested", exactBuild: null },
        ],
        compatibilityExpanded: false,
      },
      null,
      2,
    ),
  );
  if (!passed) process.exitCode = 1;
} catch {
  console.error("Cross-build baseline failed; no qualifying report produced.");
  process.exitCode = 1;
}
