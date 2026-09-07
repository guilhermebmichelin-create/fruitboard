#!/usr/bin/env node
//
// Synthetic fixture tree generator for #41 / P2-11 prep.
//
// Purpose: deterministically create the accepted baseline fixture (10,000
// FLP-named synthetic files across 1,000 leaf directories) and the
// qualification set (100,000 FLP-named files across 10,000 leaf directories)
// defined in docs/PHASE_2_EXECUTION_PLAN.md, section "Proposed targets
// accepted as provisional budgets". The trees include Unicode names, long
// paths near the Windows MAX_PATH limit, hardlink alias cases (Windows-only;
// skipped gracefully elsewhere), empty directories and nested directories.
//
// File contents are synthetic bytes carrying a plain-text marker stating that
// they are not FL Studio projects. No real FLP content and no private data is
// ever embedded, and the manifest stores root-relative paths only.
//
// Operational cost observed while preparing this scaffold (local NVMe volume;
// this is the cost of generating fixtures, not a scanner measurement):
// baseline (10,000 files) ~3 s and ~26 MB on disk; qualification (100,000
// files) ~36 s and ~262 MB on disk. Exact times vary by host and are
// recorded by the operator; they are not benchmark results.
//
// The manifest hash is deterministic for a given seed and hardlink support.
// Hardlink support differs by platform ("created" on Windows, "skipped"
// elsewhere), so cross-platform hashes may differ by design.
//
// Usage:
//   node scripts/generate-synthetic-tree.mjs --size baseline --seed 0 --out <dir>
//   node scripts/generate-synthetic-tree.mjs --size qualification --seed 0 --out <dir>
//
// The generator refuses to write anything outside --out, refuses filesystem
// roots and the OS home directory, and requires an empty (or absent) output
// directory.

import { createHash } from "node:crypto";
import { existsSync, readdirSync, statSync } from "node:fs";
import { link, mkdir, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

export const MANIFEST_SCHEMA = "fruitboard/synthetic-tree-manifest/1";
export const MANIFEST_FILENAME = "manifest.json";
export const SYNTHETIC_MARKER =
  "FRUITBOARD SYNTHETIC FIXTURE. NOT AN FL STUDIO PROJECT. NO PRIVATE DATA.\n";

export const SIZE_PRESETS = {
  baseline: { fileCount: 10_000, leafDirectoryCount: 1_000 },
  qualification: { fileCount: 100_000, leafDirectoryCount: 10_000 },
};

const ASCII_STEMS = [
  "sunset-beat",
  "chorus-draft",
  "night-loop",
  "tape-sketch",
  "drum-take",
  "mix-reference",
  "pad-layer",
  "groove-idea",
];

const UNICODE_STEMS = [
  "Découpage",
  "Übersicht",
  "Σύνθεση",
  "ノート",
  "микс",
  "Fluxo",
  "Água",
  "日本語",
  "καλημέρα",
  "Волна",
  "Éclair",
  "Øresund",
  "☔-intro",
  "カフェ",
  "Тень",
  "Bøjning",
  "Fleuré",
  "Órbita",
];

const LONG_SEGMENT = `level-${"m".repeat(26)}`;
const LONG_CHAIN_SEGMENTS = 6;
const NESTED_SPINE_LEVELS = 3;
const FILLER_BYTES = 4_096;
const MAX_BYTES = 4_096;
const MIN_BYTES = 1_024;
const HARDLINK_GROUP_CAP = 50;

const pad = (value, width) => String(value).padStart(width, "0");
const byPath = (left, right) =>
  left.path < right.path ? -1 : left.path > right.path ? 1 : 0;

const posixJoin = (...segments) => segments.filter(Boolean).join("/");

function seedToUint32(seed) {
  const text = String(seed);
  if (/^-?\d+$/u.test(text)) {
    return Number.parseInt(text, 10) >>> 0;
  }
  let hash = 0x811c9dc5;
  for (const character of Buffer.from(text, "utf8")) {
    hash ^= character;
    hash = Math.imul(hash, 0x01000193);
  }
  return hash >>> 0;
}

export function createRng(seed) {
  let state = seedToUint32(seed);
  if (state === 0) {
    state = 0x9e3779b9;
  }
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let mixed = Math.imul(state ^ (state >>> 15), state | 1);
    mixed ^= mixed + Math.imul(mixed ^ (mixed >>> 7), mixed | 61);
    return ((mixed ^ (mixed >>> 14)) >>> 0) / 4294967296;
  };
}

const pickStem = (rng, stems) => stems[Math.floor(rng() * stems.length)];

function buildDirectoryBuckets(leafDirectoryCount) {
  const longChainBranches = Math.max(1, Math.floor(leafDirectoryCount / 20));
  const nestedLeaves = Math.max(1, Math.floor(leafDirectoryCount / 20));
  const unicodeLeaves = Math.max(1, Math.floor(leafDirectoryCount / 10));
  const asciiLeaves =
    leafDirectoryCount - longChainBranches - nestedLeaves - unicodeLeaves;
  if (asciiLeaves < 1) {
    throw new Error(
      `leafDirectoryCount ${leafDirectoryCount} is too small to cover the required layout buckets`,
    );
  }
  return { longChainBranches, nestedLeaves, unicodeLeaves, asciiLeaves };
}

function buildDirectoryPaths(leafDirectoryCount) {
  const { longChainBranches, nestedLeaves, unicodeLeaves, asciiLeaves } =
    buildDirectoryBuckets(leafDirectoryCount);
  const directories = [
    "roots",
    "unicode",
    "long",
    "nested",
    "empty",
    "aliases",
  ];
  const leaves = [];

  for (let depth = 1; depth <= LONG_CHAIN_SEGMENTS; depth += 1) {
    directories.push(
      posixJoin("long", ...Array.from({ length: depth }, () => LONG_SEGMENT)),
    );
  }
  for (let depth = 1; depth <= NESTED_SPINE_LEVELS; depth += 1) {
    directories.push(
      posixJoin(
        "nested",
        ...Array.from({ length: depth }, (_, level) => `level-${level + 1}`),
      ),
    );
  }

  for (let index = 0; index < asciiLeaves; index += 1) {
    leaves.push({
      path: posixJoin(
        "roots",
        `shard-${pad(index, 4)}-${ASCII_STEMS[index % ASCII_STEMS.length]}`,
      ),
      style: "ascii",
    });
  }
  for (let index = 0; index < unicodeLeaves; index += 1) {
    leaves.push({
      path: posixJoin(
        "unicode",
        `shard-${pad(index, 4)}-${UNICODE_STEMS[index % UNICODE_STEMS.length]}`,
      ),
      style: "unicode",
    });
  }
  const longPrefix = posixJoin(
    "long",
    ...Array.from({ length: LONG_CHAIN_SEGMENTS }, () => LONG_SEGMENT),
  );
  for (let index = 0; index < longChainBranches; index += 1) {
    leaves.push({
      path: posixJoin(longPrefix, `branch-${pad(index, 4)}`),
      style: "long",
    });
  }
  const nestedPrefix = posixJoin(
    "nested",
    ...Array.from(
      { length: NESTED_SPINE_LEVELS },
      (_, level) => `level-${level + 1}`,
    ),
  );
  for (let index = 0; index < nestedLeaves; index += 1) {
    leaves.push({
      path: posixJoin(nestedPrefix, `leaf-${pad(index, 4)}`),
      style: "nested",
    });
  }

  const emptyDirectoryCount = Math.max(2, Math.floor(leafDirectoryCount / 100));
  for (let index = 0; index < emptyDirectoryCount; index += 1) {
    directories.push(posixJoin("empty", `empty-${pad(index, 4)}`));
  }

  for (const leaf of leaves) {
    directories.push(leaf.path);
  }

  return {
    directories: directories.sort(),
    leaves,
    emptyDirectoryCount,
  };
}

const extensionForSlot = (slot) => {
  if (slot % 10 === 8) {
    return ".FLP";
  }
  if (slot % 10 === 9) {
    return ".Flp";
  }
  return ".flp";
};

const decoyForLeaf = (leafOrdinal) => {
  const style = Math.floor(leafOrdinal / 250) % 3;
  const suffix = pad(leafOrdinal, 4);
  if (style === 0) {
    return { name: `readme-notes-${suffix}.txt`, kind: "other" };
  }
  if (style === 1) {
    return { name: `project-notes-${suffix}`, kind: "other" };
  }
  return { name: `backup-copy-${suffix}.flpx`, kind: "other" };
};

export function buildPlan({
  sizeLabel = "custom",
  seed = "0",
  fileCount,
  leafDirectoryCount,
}) {
  if (!Number.isInteger(fileCount) || fileCount < 1) {
    throw new Error("fileCount must be a positive integer");
  }
  if (!Number.isInteger(leafDirectoryCount) || leafDirectoryCount < 1) {
    throw new Error("leafDirectoryCount must be a positive integer");
  }
  if (fileCount < leafDirectoryCount) {
    throw new Error("fileCount must be at least leafDirectoryCount");
  }

  const rng = createRng(seed);
  const { directories, leaves, emptyDirectoryCount } =
    buildDirectoryPaths(leafDirectoryCount);

  const entries = [];
  const baseFilesPerLeaf = Math.floor(fileCount / leafDirectoryCount);
  const remainder = fileCount % leafDirectoryCount;

  leaves.forEach((leaf, leafOrdinal) => {
    const filesForLeaf = baseFilesPerLeaf + (leafOrdinal < remainder ? 1 : 0);
    const stems = leaf.style === "unicode" ? UNICODE_STEMS : ASCII_STEMS;
    for (let slot = 0; slot < filesForLeaf; slot += 1) {
      const stem = pickStem(rng, stems);
      entries.push({
        path: posixJoin(
          leaf.path,
          `${stem}-${pad(leafOrdinal, 4)}-${pad(slot, 2)}${extensionForSlot(slot)}`,
        ),
        bytes: MIN_BYTES + Math.floor(rng() * (MAX_BYTES - MIN_BYTES)),
        kind: "flp",
        hardlinkGroup: null,
        hardlinkRole: null,
      });
    }
    if (leafOrdinal % 250 === 0) {
      const decoy = decoyForLeaf(leafOrdinal);
      entries.push({
        path: posixJoin(leaf.path, decoy.name),
        bytes: 2_048 + Math.floor(rng() * 1_024),
        kind: decoy.kind,
        hardlinkGroup: null,
        hardlinkRole: null,
      });
    }
  });
  entries.sort(byPath);

  const hardlinkGroupCount = Math.min(
    HARDLINK_GROUP_CAP,
    Math.max(1, Math.floor(leafDirectoryCount / 200)),
  );
  const hardlinkGroups = [];
  for (let group = 0; group < hardlinkGroupCount; group += 1) {
    const leaf = leaves[(group * 37) % leaves.length];
    const primary = entries.find(
      (entry) =>
        entry.kind === "flp" &&
        entry.hardlinkRole === null &&
        entry.path.startsWith(`${leaf.path}/`),
    );
    if (!primary) {
      throw new Error(`no primary file available for hardlink group ${group}`);
    }
    const groupId = `hl-${pad(group + 1, 3)}`;
    primary.hardlinkGroup = groupId;
    primary.hardlinkRole = "primary";
    hardlinkGroups.push({
      id: groupId,
      primaryPath: primary.path,
      aliasPath: posixJoin(
        "aliases",
        `hardlink-alias-${pad(group + 1, 3)}.flp`,
      ),
      bytes: primary.bytes,
    });
  }

  return {
    schema: MANIFEST_SCHEMA,
    sizeLabel,
    seed: String(seed),
    hardlinkSupport: "pending",
    counts: {
      flpFiles: entries.filter(
        (entry) => entry.kind === "flp" && entry.hardlinkRole !== "alias",
      ).length,
      otherFiles: entries.filter((entry) => entry.kind === "other").length,
      aliasLocations: 0,
      leafDirectories: leaves.length,
      directories: directories.length,
      emptyDirectories: emptyDirectoryCount,
      hardlinkGroups: hardlinkGroupCount,
    },
    entries,
    hardlinkGroups,
    directories,
  };
}

export function canonicalManifestJson(manifest) {
  return JSON.stringify({
    schema: manifest.schema,
    sizeLabel: manifest.sizeLabel,
    seed: manifest.seed,
    hardlinkSupport: manifest.hardlinkSupport,
    counts: manifest.counts,
    entries: [...manifest.entries].sort(byPath),
  });
}

export function computeManifestHash(manifest) {
  return createHash("sha256")
    .update(canonicalManifestJson(manifest))
    .digest("hex");
}

export function validateSyntheticManifest(value, { allowPending = true } = {}) {
  const errors = [];
  const fail = (message) => errors.push(message);
  const isPlainObject = (candidate) =>
    typeof candidate === "object" &&
    candidate !== null &&
    !Array.isArray(candidate);

  if (!isPlainObject(value)) {
    return { ok: false, errors: ["manifest must be a JSON object"] };
  }
  if (value.schema !== MANIFEST_SCHEMA) {
    fail(`schema must be ${MANIFEST_SCHEMA}`);
  }
  if (typeof value.sizeLabel !== "string" || value.sizeLabel.length === 0) {
    fail("sizeLabel must be a non-empty string");
  }
  if (typeof value.seed !== "string" || value.seed.length === 0) {
    fail("seed must be a non-empty string");
  }
  if (
    !["created", "skipped", "unavailable", "pending"].includes(
      value.hardlinkSupport,
    )
  ) {
    fail("hardlinkSupport must be created, skipped or unavailable");
  }
  if (!isPlainObject(value.counts)) {
    fail("counts must be an object");
  }
  if (!Array.isArray(value.entries) || value.entries.length === 0) {
    fail("entries must be a non-empty array");
  }

  const counts = value.counts ?? {};
  const entries = value.entries ?? [];
  for (const key of [
    "flpFiles",
    "otherFiles",
    "aliasLocations",
    "leafDirectories",
    "directories",
    "emptyDirectories",
    "hardlinkGroups",
  ]) {
    if (!Number.isInteger(counts[key]) || counts[key] < 0) {
      fail(`counts.${key} must be a non-negative integer`);
    }
  }
  if (
    Number.isInteger(counts.leafDirectories) &&
    Number.isInteger(counts.directories) &&
    counts.directories < counts.leafDirectories
  ) {
    fail("counts.directories must cover counts.leafDirectories");
  }

  const groups = new Map();
  let previousPath = null;
  for (const [index, entry] of entries.entries()) {
    const label = `entries[${index}]`;
    if (!isPlainObject(entry)) {
      fail(`${label} must be an object`);
      continue;
    }
    const { path: entryPath, bytes, kind, hardlinkGroup, hardlinkRole } = entry;
    if (typeof entryPath !== "string" || entryPath.length === 0) {
      fail(`${label}.path must be a non-empty string`);
      continue;
    }
    if (entryPath.includes("\\")) {
      fail(`${label}.path must use posix separators: ${entryPath}`);
    }
    if (entryPath.startsWith("/") || /^[A-Za-z]:/u.test(entryPath)) {
      fail(`${label}.path must be relative: ${entryPath}`);
    }
    if (
      entryPath.split("/").some((segment) => segment === ".." || segment === "")
    ) {
      fail(`${label}.path must stay inside the fixture root: ${entryPath}`);
    }
    if (previousPath !== null && entryPath <= previousPath) {
      fail(`${label}.path must be strictly ascending: ${entryPath}`);
    }
    previousPath = entryPath;

    if (!Number.isInteger(bytes) || bytes < 1 || bytes > 1_048_576) {
      fail(`${label}.bytes must be an integer between 1 and 1048576`);
    }
    if (kind !== "flp" && kind !== "other") {
      fail(`${label}.kind must be flp or other`);
    } else if (kind === "flp" && !entryPath.toLowerCase().endsWith(".flp")) {
      fail(`${label} must carry a case-insensitive .flp name`);
    } else if (kind === "other" && entryPath.toLowerCase().endsWith(".flp")) {
      fail(`${label} is marked other but carries an .flp name`);
    }
    if (
      hardlinkRole !== null &&
      hardlinkRole !== undefined &&
      hardlinkRole !== "primary" &&
      hardlinkRole !== "alias"
    ) {
      fail(`${label}.hardlinkRole must be primary, alias or null`);
    }
    if (hardlinkRole === "primary" || hardlinkRole === "alias") {
      if (typeof hardlinkGroup !== "string" || hardlinkGroup.length === 0) {
        fail(`${label} with a hardlink role needs a hardlinkGroup`);
      } else {
        const group = groups.get(hardlinkGroup) ?? [];
        group.push(entry);
        groups.set(hardlinkGroup, group);
      }
    } else if (hardlinkGroup !== null && hardlinkGroup !== undefined) {
      fail(`${label} without a hardlink role must not declare a group`);
    }
  }

  if (value.hardlinkSupport === "created") {
    if (groups.size === 0) {
      fail("hardlinkSupport created requires at least one alias group");
    }
    for (const [groupId, groupEntries] of groups) {
      if (groupEntries.length !== 2) {
        fail(`hardlink group ${groupId} must have exactly two locations`);
        continue;
      }
      const [first, second] = groupEntries;
      if (first.bytes !== second.bytes) {
        fail(`hardlink group ${groupId} locations must share one size`);
      }
      if (first.hardlinkRole === second.hardlinkRole) {
        fail(
          `hardlink group ${groupId} needs one primary and one alias location`,
        );
      }
    }
  } else if (value.hardlinkSupport === "pending") {
    if (!allowPending) {
      fail("hardlinkSupport pending is not a finished fixture manifest");
    }
    for (const [index, entry] of entries.entries()) {
      if (entry.hardlinkRole === "alias") {
        fail(
          `entries[${index}].hardlinkRole alias requires created hardlink support`,
        );
      }
    }
  } else {
    for (const [index, entry] of entries.entries()) {
      if (entry.hardlinkRole !== null && entry.hardlinkRole !== undefined) {
        fail(
          `entries[${index}].hardlinkRole must be null without created hardlink support`,
        );
      }
    }
  }

  if (
    Number.isInteger(counts.flpFiles) &&
    Number.isInteger(counts.aliasLocations) &&
    Number.isInteger(counts.otherFiles) &&
    errors.length === 0
  ) {
    const aliasLocations = entries.filter(
      (entry) => entry.hardlinkRole === "alias",
    ).length;
    const flpFiles = entries.filter(
      (entry) => entry.kind === "flp" && entry.hardlinkRole !== "alias",
    ).length;
    const otherFiles = entries.filter((entry) => entry.kind === "other").length;
    if (aliasLocations !== counts.aliasLocations) {
      fail("counts.aliasLocations does not match entries");
    }
    if (flpFiles !== counts.flpFiles) {
      fail("counts.flpFiles does not match entries");
    }
    if (otherFiles !== counts.otherFiles) {
      fail("counts.otherFiles does not match entries");
    }
  }

  return { ok: errors.length === 0, errors };
}

export function assertInsideOut(outAbsolute, targetAbsolute) {
  const relative = path.relative(outAbsolute, targetAbsolute);
  if (relative === "") {
    throw new Error("refusing to write the output directory itself");
  }
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new Error(
      "refusing to write outside the output directory (traversal detected)",
    );
  }
  return targetAbsolute;
}

const toExtendedLengthPath = (absolutePath) => {
  if (
    process.platform === "win32" &&
    !absolutePath.startsWith("\\\\?\\") &&
    /^[A-Za-z]:[\\/]/u.test(absolutePath)
  ) {
    return `\\\\?\\${absolutePath.replaceAll("/", "\\")}`;
  }
  return absolutePath;
};

async function runPool(items, limit, worker) {
  let cursor = 0;
  const runners = Array.from(
    { length: Math.min(limit, items.length) },
    async () => {
      while (cursor < items.length) {
        const index = cursor;
        cursor += 1;
        await worker(items[index], index);
      }
    },
  );
  await Promise.all(runners);
}

function resolveOutputDirectory(outDir) {
  const outAbsolute = path.resolve(outDir);
  const { root } = path.parse(outAbsolute);
  if (outAbsolute === root) {
    throw new Error(`refusing to use a filesystem root as --out: ${root}`);
  }
  const home = path.resolve(os.homedir());
  const sameAsHome =
    process.platform === "win32"
      ? home.toLowerCase() === outAbsolute.toLowerCase()
      : home === outAbsolute;
  if (sameAsHome) {
    throw new Error("refusing to use the OS home directory as --out");
  }
  if (existsSync(outAbsolute)) {
    const stat = statSync(outAbsolute);
    if (!stat.isDirectory()) {
      throw new Error(`--out exists and is not a directory: ${outDir}`);
    }
    if (readdirSync(outAbsolute).length > 0) {
      throw new Error(
        `--out exists and is not empty; choose an empty directory: ${outDir}`,
      );
    }
  }
  return outAbsolute;
}

const targetInside = (outAbsolute, relativePath) =>
  assertInsideOut(
    outAbsolute,
    path.join(outAbsolute, ...relativePath.split("/")),
  );

async function writeEntry(outAbsolute, entry, index, filler, marker) {
  const bodyLength = Math.max(0, entry.bytes - marker.length);
  const slack = FILLER_BYTES - bodyLength;
  const offset = slack > 0 ? (index * 97) % slack : 0;
  const content = Buffer.concat([
    marker,
    filler.subarray(offset, offset + bodyLength),
  ]);
  await writeFile(
    toExtendedLengthPath(targetInside(outAbsolute, entry.path)),
    content,
  );
}

const persistedManifest = (plan) => ({
  schema: plan.schema,
  sizeLabel: plan.sizeLabel,
  seed: plan.seed,
  hardlinkSupport: plan.hardlinkSupport,
  counts: plan.counts,
  entries: [...plan.entries].sort(byPath),
});

function resetHardlinkGroups(plan) {
  for (const group of plan.hardlinkGroups) {
    const primary = plan.entries.find(
      (entry) => entry.path === group.primaryPath,
    );
    if (primary) {
      primary.hardlinkGroup = null;
      primary.hardlinkRole = null;
    }
  }
}

export async function writePlan(plan, outDir) {
  const validated = validateSyntheticManifest(plan);
  if (!validated.ok) {
    throw new Error(
      `refusing to write an invalid plan: ${validated.errors[0]}`,
    );
  }
  const outAbsolute = resolveOutputDirectory(outDir);
  await mkdir(toExtendedLengthPath(outAbsolute), { recursive: true });

  await runPool(plan.directories, 64, async (directory) => {
    await mkdir(toExtendedLengthPath(targetInside(outAbsolute, directory)), {
      recursive: true,
    });
  });

  const rng = createRng(`${plan.seed}:filler`);
  const filler = Buffer.alloc(FILLER_BYTES);
  for (let index = 0; index < FILLER_BYTES; index += 1) {
    filler[index] = Math.floor(rng() * 256);
  }
  const marker = Buffer.from(SYNTHETIC_MARKER, "utf8");

  await runPool(plan.entries, 64, (entry, index) =>
    writeEntry(outAbsolute, entry, index, filler, marker),
  );

  if (process.platform === "win32") {
    let allLinked = true;
    await runPool(plan.hardlinkGroups, 8, async (group) => {
      const primary = targetInside(outAbsolute, group.primaryPath);
      const alias = targetInside(outAbsolute, group.aliasPath);
      try {
        await link(toExtendedLengthPath(primary), toExtendedLengthPath(alias));
      } catch (error) {
        allLinked = false;
        console.warn(
          `hardlink group ${group.id} could not be created on this volume: ${error.code ?? error.message}`,
        );
      }
    });
    if (allLinked) {
      plan.hardlinkSupport = "created";
      plan.counts.aliasLocations = plan.hardlinkGroups.length;
      for (const group of plan.hardlinkGroups) {
        plan.entries.push({
          path: group.aliasPath,
          bytes: group.bytes,
          kind: "flp",
          hardlinkGroup: group.id,
          hardlinkRole: "alias",
        });
      }
    } else {
      plan.hardlinkSupport = "unavailable";
      resetHardlinkGroups(plan);
    }
  } else {
    plan.hardlinkSupport = "skipped";
    resetHardlinkGroups(plan);
  }
  plan.entries.sort(byPath);

  const hash = computeManifestHash(plan);
  const manifestPath = targetInside(outAbsolute, MANIFEST_FILENAME);
  await writeFile(
    toExtendedLengthPath(manifestPath),
    `${JSON.stringify(persistedManifest(plan), null, 2)}\n`,
    "utf8",
  );

  return { outAbsolute, hash, manifestPath };
}

export function parseCliArgs(argv) {
  const usage = [
    "Usage:",
    "  node scripts/generate-synthetic-tree.mjs --size <baseline|qualification> --seed <seed> --out <dir>",
    "",
    "Options:",
    "  --size baseline|qualification  fixture preset from the accepted Phase 2 budgets",
    "  --seed <seed>                  deterministic seed (numeric or text)",
    "  --out <dir>                    empty output directory; the generator never writes outside it",
    "  --help                         show this help",
  ].join("\n");
  const parsed = { size: null, seed: "0", out: null, help: false };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const equals = argument.indexOf("=");
    const flag = equals === -1 ? argument : argument.slice(0, equals);
    const inlineValue = equals === -1 ? undefined : argument.slice(equals + 1);
    const next = () => {
      if (inlineValue !== undefined) {
        return inlineValue;
      }
      index += 1;
      if (index >= argv.length) {
        throw new Error(`missing value for ${flag}\n${usage}`);
      }
      return argv[index];
    };
    switch (flag) {
      case "--size":
        parsed.size = next();
        break;
      case "--seed":
        parsed.seed = next();
        break;
      case "--out":
        parsed.out = next();
        break;
      case "--help":
      case "-h":
        parsed.help = true;
        break;
      default:
        throw new Error(`unknown argument: ${argument}\n${usage}`);
    }
  }
  if (parsed.help) {
    return parsed;
  }
  if (parsed.size === null) {
    throw new Error(`--size is required (baseline|qualification)\n${usage}`);
  }
  if (!Object.hasOwn(SIZE_PRESETS, parsed.size)) {
    throw new Error(
      `--size must be one of ${Object.keys(SIZE_PRESETS).join("|")}\n${usage}`,
    );
  }
  if (parsed.out === null) {
    throw new Error(`--out is required\n${usage}`);
  }
  return parsed;
}

function summarize(manifest) {
  return [
    `synthetic tree: ${manifest.sizeLabel}`,
    `seed: ${manifest.seed}`,
    `directories: ${manifest.counts.directories} (${manifest.counts.leafDirectories} leaf, ${manifest.counts.emptyDirectories} empty)`,
    `files: ${manifest.counts.flpFiles} .flp-named (+${manifest.counts.otherFiles} other)`,
    `hardlink groups: ${manifest.counts.hardlinkGroups} (support: ${manifest.hardlinkSupport})`,
  ];
}

export async function main(argv = process.argv.slice(2)) {
  let parsed;
  try {
    parsed = parseCliArgs(argv);
  } catch (error) {
    console.error(error.message);
    return 1;
  }
  if (parsed.help) {
    console.log(
      "Usage: node scripts/generate-synthetic-tree.mjs --size <baseline|qualification> --seed <seed> --out <dir>",
    );
    return 0;
  }
  try {
    const preset = SIZE_PRESETS[parsed.size];
    const plan = buildPlan({
      sizeLabel: parsed.size,
      seed: parsed.seed,
      fileCount: preset.fileCount,
      leafDirectoryCount: preset.leafDirectoryCount,
    });
    const { hash, manifestPath } = await writePlan(plan, parsed.out);
    for (const line of summarize(plan)) {
      console.log(line);
    }
    console.log(`manifest: ${path.basename(manifestPath)}`);
    console.log(`manifest sha-256: ${hash}`);
    return 0;
  } catch (error) {
    console.error(`generation failed: ${error.message}`);
    return 1;
  }
}

const invokedDirectly =
  process.argv[1] !== undefined &&
  import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
  process.exitCode = await main();
}
