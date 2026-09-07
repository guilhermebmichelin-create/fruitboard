import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  assertInsideOut,
  buildPlan,
  computeManifestHash,
  parseCliArgs,
  SIZE_PRESETS,
  SYNTHETIC_MARKER,
  validateSyntheticManifest,
  writePlan,
} from "../scripts/generate-synthetic-tree.mjs";

const TINY = {
  sizeLabel: "custom",
  seed: "p2-11-test",
  fileCount: 50,
  leafDirectoryCount: 5,
};

const makeTempDir = (label) =>
  mkdtemp(path.join(os.tmpdir(), `fruitboard-synthetic-${label}-`));
const cleanup = (dir) => rm(dir, { recursive: true, force: true });
const readManifest = async (dir) =>
  JSON.parse(await readFile(path.join(dir, "manifest.json"), "utf8"));

test("the same seed produces the same manifest hash and document", async (t) => {
  const firstDir = await makeTempDir("determinism-a");
  const secondDir = await makeTempDir("determinism-b");
  t.after(() => Promise.all([cleanup(firstDir), cleanup(secondDir)]));

  const first = await writePlan(buildPlan(TINY), firstDir);
  const second = await writePlan(buildPlan(TINY), secondDir);
  assert.equal(first.hash, second.hash);
  assert.equal(
    await readFile(path.join(firstDir, "manifest.json"), "utf8"),
    await readFile(path.join(secondDir, "manifest.json"), "utf8"),
  );
  assert.equal(first.hash.length, 64);
  const document = await readManifest(firstDir);
  assert.equal(computeManifestHash(document), first.hash);
  assert.doesNotMatch(
    await readFile(path.join(firstDir, "manifest.json"), "utf8"),
    /mtime/i,
    "manifests must not embed wall-clock timestamps",
  );
});

test("generated manifests stay relative and free of personal paths", async (t) => {
  const dir = await makeTempDir("privacy");
  t.after(() => cleanup(dir));
  await writePlan(buildPlan(TINY), dir);
  const text = await readFile(path.join(dir, "manifest.json"), "utf8");

  assert.doesNotMatch(text, /[A-Za-z]:[\\/]/u, "no drive-letter paths");
  assert.doesNotMatch(text, /\/(?:Users|home)\//u, "no POSIX home paths");
  assert.doesNotMatch(text, /\\\\/u, "no windows separators");
  const document = JSON.parse(text);
  for (const entry of document.entries) {
    assert.equal(path.posix.isAbsolute(entry.path), false);
    assert.equal(entry.path.split("/").includes(".."), false);
  }
  assert.equal(document.seed, TINY.seed);
});

test("the generator refuses writes outside its output directory", async (t) => {
  const dir = await makeTempDir("containment");
  const parent = path.dirname(dir);
  t.after(() => cleanup(dir));

  assert.throws(
    () => assertInsideOut(dir, path.join(dir, "..", "escape.flp")),
    /outside the output directory/,
  );
  assert.throws(
    () => assertInsideOut(dir, path.join(dir, "roots", "..", "..", "x.flp")),
    /outside the output directory/,
  );

  const malicious = buildPlan(TINY);
  malicious.entries.push({
    path: "../escape.flp",
    bytes: 16,
    kind: "other",
    hardlinkGroup: null,
    hardlinkRole: null,
  });
  await assert.rejects(
    () => writePlan(malicious, dir),
    /refusing to write an invalid plan/,
  );
  assert.equal(existsSync(path.join(parent, "escape.flp")), false);

  await assert.rejects(
    () => writePlan(buildPlan(TINY), os.homedir()),
    /refusing/,
  );
  const root = process.platform === "win32" ? "C:\\" : "/";
  await assert.rejects(() => writePlan(buildPlan(TINY), root), /refusing/);
});

test(".flp matching is case-insensitive on content-agnostic names", async (t) => {
  const dir = await makeTempDir("case-insensitive");
  t.after(() => cleanup(dir));
  await writePlan(buildPlan(TINY), dir);
  const document = await readManifest(dir);

  const flpEntries = document.entries.filter((entry) => entry.kind === "flp");
  const otherEntries = document.entries.filter(
    (entry) => entry.kind === "other",
  );
  assert.ok(flpEntries.length > 0);
  for (const entry of flpEntries) {
    assert.equal(entry.path.toLowerCase().endsWith(".flp"), true, entry.path);
  }
  const extensions = new Set(
    flpEntries.map((entry) => entry.path.slice(entry.path.lastIndexOf("."))),
  );
  for (const extension of [".flp", ".FLP", ".Flp"]) {
    assert.equal(
      extensions.has(extension),
      true,
      `missing variant ${extension}`,
    );
  }
  assert.ok(otherEntries.length > 0, "content-agnostic decoys must exist");
  for (const entry of otherEntries) {
    assert.equal(entry.path.toLowerCase().endsWith(".flp"), false, entry.path);
  }

  const sample = flpEntries[0];
  const raw = await readFile(path.join(dir, ...sample.path.split("/")));
  assert.equal(
    raw.toString("utf8", 0, SYNTHETIC_MARKER.length),
    SYNTHETIC_MARKER,
    "fixture files must identify themselves as synthetic",
  );
  assert.equal(
    raw.toString("latin1").includes("FLhd"),
    false,
    "fixture files must not carry real FLP content",
  );
});

test("hardlink alias cases share one identity across two locations", async (t) => {
  const dir = await makeTempDir("hardlinks");
  t.after(() => cleanup(dir));
  await writePlan(buildPlan(TINY), dir);
  const document = await readManifest(dir);

  if (document.hardlinkSupport === "created") {
    const groups = new Map();
    for (const entry of document.entries) {
      if (entry.hardlinkGroup !== null) {
        const group = groups.get(entry.hardlinkGroup) ?? [];
        group.push(entry);
        groups.set(entry.hardlinkGroup, group);
      }
    }
    assert.ok(groups.size >= 1);
    for (const [groupId, groupEntries] of groups) {
      assert.equal(groupEntries.length, 2, groupId);
      const [first, second] = groupEntries;
      assert.notEqual(first.path, second.path, groupId);
      assert.equal(first.bytes, second.bytes, groupId);
      assert.equal(first.kind, "flp", groupId);
      assert.deepEqual(
        new Set([first.hardlinkRole, second.hardlinkRole]),
        new Set(["primary", "alias"]),
        groupId,
      );
    }
    assert.equal(document.counts.aliasLocations, groups.size);
  } else {
    assert.equal(document.hardlinkSupport, "skipped");
    assert.equal(process.platform !== "win32", true);
    assert.equal(document.counts.aliasLocations, 0);
  }
});

test("preset plans match the accepted fixture targets", () => {
  const baseline = buildPlan({
    sizeLabel: "baseline",
    seed: "p2-11",
    ...SIZE_PRESETS.baseline,
  });
  const baselineFlp = baseline.entries.filter(
    (entry) => entry.kind === "flp" && entry.hardlinkRole !== "alias",
  );
  assert.equal(baselineFlp.length, 10_000);
  assert.equal(baseline.counts.leafDirectories, 1_000);
  assert.equal(
    baseline.counts.leafDirectories,
    SIZE_PRESETS.baseline.leafDirectoryCount,
  );

  const qualification = buildPlan({
    sizeLabel: "qualification",
    seed: "p2-11",
    ...SIZE_PRESETS.qualification,
  });
  const qualificationFlp = qualification.entries.filter(
    (entry) => entry.kind === "flp" && entry.hardlinkRole !== "alias",
  );
  assert.equal(qualificationFlp.length, 100_000);
  assert.equal(qualification.counts.leafDirectories, 10_000);

  for (const plan of [baseline, qualification]) {
    assert.ok(
      plan.entries.some((entry) => /[^\u0000-\u007f]/u.test(entry.path)),
      "fixture must cover Unicode names",
    );
    assert.ok(
      plan.entries.some((entry) => entry.path.length >= 200),
      "fixture must cover long paths near MAX_PATH",
    );
    assert.ok(
      plan.counts.emptyDirectories >= 2,
      "fixture must cover empty dirs",
    );
  }
  const validated = validateSyntheticManifest(baseline);
  assert.equal(validated.ok, true, validated.errors.join("; "));
});

test("the CLI contract stays pinned to the accepted presets", () => {
  assert.deepEqual(SIZE_PRESETS.baseline, {
    fileCount: 10_000,
    leafDirectoryCount: 1_000,
  });
  assert.deepEqual(SIZE_PRESETS.qualification, {
    fileCount: 100_000,
    leafDirectoryCount: 10_000,
  });
  const parsed = parseCliArgs([
    "--size=baseline",
    "--seed=p2-11",
    "--out=%TEMP%\\fixture",
  ]);
  assert.deepEqual(parsed, {
    size: "baseline",
    seed: "p2-11",
    out: "%TEMP%\\fixture",
    help: false,
  });
  assert.throws(
    () => parseCliArgs(["--size", "baseline"]),
    /--out is required/,
  );
  assert.throws(() => parseCliArgs(["--out", "x"]), /--size is required/);
  assert.throws(
    () => parseCliArgs(["--size", "huge", "--out", "x"]),
    /--size must be one of/,
  );
  assert.throws(() => parseCliArgs(["--verbose"]), /unknown argument/);
});

test("manifest validation rejects tampered fixtures", () => {
  const document = buildPlan(TINY);
  document.hardlinkSupport = "skipped";
  for (const entry of document.entries) {
    entry.hardlinkGroup = null;
    entry.hardlinkRole = null;
  }
  assert.equal(validateSyntheticManifest(document).ok, true);

  const absolute = structuredClone(document);
  absolute.entries[0].path = `${path.parse(os.tmpdir()).root}abs.flp`;
  assert.match(
    validateSyntheticManifest(absolute).errors.join("\n"),
    /must be relative/,
  );

  const unsorted = structuredClone(document);
  unsorted.entries.reverse();
  assert.match(
    validateSyntheticManifest(unsorted).errors.join("\n"),
    /strictly ascending/,
  );

  const miscounted = structuredClone(document);
  miscounted.counts.flpFiles += 1;
  assert.match(
    validateSyntheticManifest(miscounted).errors.join("\n"),
    /counts\.flpFiles does not match entries/,
  );

  const incompleteAliases = structuredClone(document);
  incompleteAliases.hardlinkSupport = "created";
  assert.match(
    validateSyntheticManifest(incompleteAliases).errors.join("\n"),
    /requires at least one alias group/,
  );
});
