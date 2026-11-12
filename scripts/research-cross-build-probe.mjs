#!/usr/bin/env node
// Supervise the existing no-argument, fixed-control probe; never accept FLPs.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  closeSync,
  fstatSync,
  lstatSync,
  openSync,
  readFileSync,
  readSync,
  realpathSync,
} from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { compareFixedProbe } from "./lib/cross-build-probe.mjs";

const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const specificationUrl = new URL(
  "../research/cross-build-controls.json",
  import.meta.url,
);

function snapshot(path, expectedHash, expectedSize, maxBytes) {
  if (lstatSync(path).isSymbolicLink() || realpathSync(path) !== path)
    throw new Error("RESEARCH_PATH");
  const fd = openSync(path, "r");
  try {
    const before = fstatSync(fd);
    if (
      !before.isFile() ||
      before.size > maxBytes ||
      (expectedSize !== null && before.size !== expectedSize)
    )
      throw new Error("RESEARCH_SIZE");
    const bytes = Buffer.alloc(before.size + 1);
    let size = 0;
    while (size < bytes.length) {
      const count = readSync(fd, bytes, size, bytes.length - size, size);
      if (!count) break;
      size += count;
    }
    const after = fstatSync(fd);
    if (
      size !== before.size ||
      after.size !== before.size ||
      after.mtimeMs !== before.mtimeMs ||
      hash(bytes.subarray(0, size)) !== expectedHash
    )
      throw new Error("RESEARCH_CHANGED");
    return {
      device: before.dev,
      inode: before.ino,
      modified: before.mtimeMs,
      size,
    };
  } finally {
    closeSync(fd);
  }
}

try {
  const args = process.argv.slice(2);
  if (
    args.length !== 4 ||
    args[0] !== "--probe" ||
    args[2] !== "--sha256" ||
    !/^[a-f0-9]{64}$/.test(args[3])
  )
    throw new Error("RESEARCH_ARGUMENTS");
  const probe = realpathSync(args[1]);
  const root = realpathSync(
    fileURLToPath(new URL("../fixtures/parser-corpus", import.meta.url)),
  );
  const specificationBytes = readFileSync(specificationUrl);
  const specification = JSON.parse(specificationBytes);
  const inputs = [
    { path: probe, sha256: args[3], bytes: null, limit: 128 * 1024 * 1024 },
    ...specification.controls.map((control) => ({
      path: join(root, control.file),
      sha256: control.sha256,
      bytes: control.bytes,
      limit: 128 * 1024,
    })),
  ];
  const before = inputs.map((input) =>
    snapshot(input.path, input.sha256, input.bytes, input.limit),
  );
  const child = spawnSync(probe, [], {
    encoding: "utf8",
    timeout: 10000,
    maxBuffer: 256 * 1024,
    windowsHide: true,
    shell: false,
  });
  for (const [i, input] of inputs.entries()) {
    const after = snapshot(input.path, input.sha256, input.bytes, input.limit);
    if (JSON.stringify(after) !== JSON.stringify(before[i]))
      throw new Error("RESEARCH_CHANGED");
  }
  if (child.error || child.status !== 0 || child.stderr.length > 64 * 1024)
    throw new Error("RESEARCH_PROCESS");
  const report = compareFixedProbe(child.stdout, specification);
  console.log(
    JSON.stringify(
      {
        ...report,
        probeSha256: args[3],
        runnerSha256: hash(readFileSync(fileURLToPath(import.meta.url))),
        comparisonSha256: hash(
          readFileSync(new URL("./lib/cross-build-probe.mjs", import.meta.url)),
        ),
        fieldComparisonSha256: hash(
          readFileSync(
            new URL("./lib/cross-build-research.mjs", import.meta.url),
          ),
        ),
        specificationSha256: hash(specificationBytes),
        sourceHashesUnchanged: true,
      },
      null,
      2,
    ),
  );
} catch {
  console.error("Cross-build probe failed; no comparison report produced.");
  process.exitCode = 1;
}
