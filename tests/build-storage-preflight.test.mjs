import assert from "node:assert/strict";
import {
  link,
  lstat,
  mkdtemp,
  mkdir,
  readFile,
  readdir,
  rm,
  stat,
  symlink,
  writeFile,
} from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  GIB,
  MIN_FREE_RESERVE_BYTES,
  parseCliArgs,
  preflightBuildStorage,
} from "../scripts/preflight-build-storage.mjs";

const makeFixture = async (t) => {
  const root = await mkdtemp(
    path.join(os.tmpdir(), "fruitboard-build-storage-preflight-"),
  );
  t.after(() => rm(root, { recursive: true, force: true }));

  const fixture = {
    root,
    source: path.join(root, "source"),
    build: path.join(root, "build"),
    cache: path.join(root, "cache"),
    evidence: path.join(root, "evidence"),
  };
  await Promise.all(
    Object.values(fixture)
      .filter((value) => value !== root)
      .map((directory) => mkdir(directory)),
  );
  await writeFile(path.join(fixture.source, "source-sentinel.txt"), "source");
  await writeFile(
    path.join(fixture.evidence, "evidence-sentinel.txt"),
    "retained evidence",
  );
  return fixture;
};

const capacity =
  (availableBytes, volumeKey = "fixture-volume") =>
  () => ({
    availableBytes,
    volumeKey,
    displayRoot: volumeKey,
  });

const validOptions = (fixture, availableBytes = 40 * GIB) => ({
  sourcePath: fixture.source,
  buildPath: fixture.build,
  cachePath: fixture.cache,
  evidencePath: fixture.evidence,
  estimatedAdditionalOutputBytes: 5 * GIB,
  readCapacity: capacity(availableBytes),
});

const failureCodes = (result) => result.failures.map(({ code }) => code);

async function protectedSnapshot(fixture) {
  const sourceSentinel = path.join(fixture.source, "source-sentinel.txt");
  const evidenceSentinel = path.join(fixture.evidence, "evidence-sentinel.txt");
  const [sourceData, evidenceData, sourceStats, evidenceStats] =
    await Promise.all([
      readFile(sourceSentinel),
      readFile(evidenceSentinel),
      lstat(sourceSentinel),
      lstat(evidenceSentinel),
    ]);
  return {
    sourceData,
    evidenceData,
    sourceStats: {
      size: sourceStats.size,
      mtimeMs: sourceStats.mtimeMs,
      ctimeMs: sourceStats.ctimeMs,
    },
    evidenceStats: {
      size: evidenceStats.size,
      mtimeMs: evidenceStats.mtimeMs,
      ctimeMs: evidenceStats.ctimeMs,
    },
  };
}

test("passes with sufficient reserve and reports the selected volume", async (t) => {
  const fixture = await makeFixture(t);
  const result = preflightBuildStorage(validOptions(fixture));

  assert.equal(result.ok, true);
  assert.deepEqual(failureCodes(result), []);
  assert.equal(result.volumes.length, 1);
  assert.deepEqual(result.volumes[0].roles.sort(), [
    "build",
    "cache",
    "evidence",
    "source",
  ]);
  assert.equal(result.volumes[0].availableBytes, 40 * GIB);
  assert.equal(result.volumes[0].estimatedAdditionalOutputBytes, 5 * GIB);
  assert.equal(result.volumes[0].remainingBytes, 35 * GIB);
  assert.equal(result.cache.status, "isolated");
});

test("fails when estimated output would breach the 30 GiB reserve", async (t) => {
  const fixture = await makeFixture(t);
  const result = preflightBuildStorage(validOptions(fixture, 34 * GIB));

  assert.equal(result.ok, false);
  assert.ok(failureCodes(result).includes("estimated-output-breach"));
  assert.equal(result.volumes[0].availableBytes, 34 * GIB);
  assert.equal(result.volumes[0].remainingBytes, 29 * GIB);
  assert.equal(result.volumes[0].reserveBytes, MIN_FREE_RESERVE_BYTES);
});

test("fails closed for an invalid location without inspecting the cache", async (t) => {
  const fixture = await makeFixture(t);
  let capacityCalls = 0;
  const result = preflightBuildStorage({
    ...validOptions(fixture),
    cachePath: path.join(fixture.root, "missing-cache"),
    readCapacity: () => {
      capacityCalls += 1;
      return { availableBytes: 40 * GIB };
    },
  });

  assert.equal(result.ok, false);
  assert.ok(failureCodes(result).includes("invalid-location"));
  assert.equal(result.cache.status, "not-checked");
  assert.equal(capacityCalls, 0);
});

test("rejects cache and evidence overlap after resolving explicit paths", async (t) => {
  const fixture = await makeFixture(t);
  const nestedCache = path.join(fixture.evidence, "nested-cache");
  await mkdir(nestedCache);
  const result = preflightBuildStorage({
    ...validOptions(fixture),
    cachePath: nestedCache,
  });

  assert.equal(result.ok, false);
  assert.ok(failureCodes(result).includes("protected-path-overlap"));
  assert.equal(result.cache.status, "not-checked");

  const sourceAsCache = preflightBuildStorage({
    ...validOptions(fixture),
    cachePath: fixture.source,
  });
  assert.ok(failureCodes(sourceAsCache).includes("cache-contains-source"));
  assert.equal(sourceAsCache.cache.status, "not-checked");
});

test("reports a hardlink alias in the selected writable cache", async (t) => {
  const fixture = await makeFixture(t);
  const cacheFile = path.join(fixture.cache, "cargo-output.exe");
  const retainedAlias = path.join(fixture.evidence, "retained.exe");
  await writeFile(cacheFile, "binary bytes");
  await link(cacheFile, retainedAlias);

  const result = preflightBuildStorage(validOptions(fixture));

  assert.equal(result.ok, false);
  assert.ok(failureCodes(result).includes("hardlink-alias"));
  assert.equal(result.cache.status, "unsafe");
  assert.equal(result.cache.hardlinkedFiles.length, 1);
  assert.equal(
    result.cache.hardlinkedFiles[0].relativePath,
    "cargo-output.exe",
  );
  assert.equal(result.cache.hardlinkedFiles[0].linkCount, 2);
});

test("does not certify a cache when its bounded scan is incomplete", async (t) => {
  const fixture = await makeFixture(t);
  await writeFile(path.join(fixture.cache, "first.bin"), "first");
  await writeFile(path.join(fixture.cache, "second.bin"), "second");

  const result = preflightBuildStorage({
    ...validOptions(fixture),
    maxCacheEntries: 1,
  });

  assert.equal(result.ok, false);
  assert.ok(failureCodes(result).includes("cache-scan-truncated"));
  assert.equal(result.cache.status, "uncertain");
});

test("leaves source and retained evidence unchanged", async (t) => {
  const fixture = await makeFixture(t);
  await writeFile(path.join(fixture.cache, "cache-entry.bin"), "cache");
  const before = await protectedSnapshot(fixture);

  const result = preflightBuildStorage(validOptions(fixture));
  const after = await protectedSnapshot(fixture);

  assert.equal(result.ok, true);
  assert.deepEqual(after, before);
  assert.deepEqual(await readdir(fixture.evidence), ["evidence-sentinel.txt"]);
});

test("resolves an explicit reparse location without rejecting unrelated links", async (t) => {
  const fixture = await makeFixture(t);
  const sourceAlias = path.join(fixture.root, "source-alias");
  const linkType = process.platform === "win32" ? "junction" : "dir";
  try {
    await symlink(fixture.source, sourceAlias, linkType);
  } catch (error) {
    t.skip(`reparse fixture unavailable: ${error.code ?? error.message}`);
    return;
  }

  const result = preflightBuildStorage({
    ...validOptions(fixture),
    sourcePath: sourceAlias,
  });

  assert.equal(result.ok, true);
  assert.equal(result.locations.source.reparsePoints.length, 1);
  assert.equal(result.warnings[0].code, "resolved-reparse-point");
});

test("fails closed when a cache reparse point escapes the selected cache", async (t) => {
  const fixture = await makeFixture(t);
  const external = path.join(fixture.root, "external-cache-data");
  await mkdir(external);
  await writeFile(path.join(external, "outside.bin"), "outside");
  const cacheLink = path.join(fixture.cache, "external-link");
  const linkType = process.platform === "win32" ? "junction" : "dir";
  try {
    await symlink(external, cacheLink, linkType);
  } catch (error) {
    t.skip(`reparse fixture unavailable: ${error.code ?? error.message}`);
    return;
  }

  const result = preflightBuildStorage(validOptions(fixture));

  assert.equal(result.ok, false);
  assert.ok(failureCodes(result).includes("cache-reparse-uncertain"));
  assert.equal(result.cache.status, "uncertain");
});

test("CLI parsing requires all explicit locations and an output estimate", () => {
  assert.throws(
    () => parseCliArgs(["--source", "source"]),
    /--build is required/,
  );
  assert.throws(
    () =>
      parseCliArgs([
        "--source",
        "source",
        "--build",
        "build",
        "--cache",
        "cache",
        "--evidence",
        "evidence",
      ]),
    /estimated-output-gib|estimated-output-bytes/,
  );
  const options = parseCliArgs([
    "--source",
    "source",
    "--build",
    "build",
    "--cache",
    "cache",
    "--evidence",
    "evidence",
    "--estimated-output-gib",
    "1.5",
  ]);
  assert.equal(options.estimatedAdditionalOutputBytes, 1.5 * GIB);
});

test("charges estimated output to a separate evidence volume", async (t) => {
  const fixture = await makeFixture(t);
  const result = preflightBuildStorage({
    ...validOptions(fixture),
    readCapacity: (_path, location) => ({
      volumeKey:
        location.role === "evidence" ? "evidence-volume" : "build-volume",
      availableBytes: (location.role === "evidence" ? 34 : 40) * GIB,
    }),
  });
  assert.equal(result.ok, false);
  assert.ok(failureCodes(result).includes("estimated-output-breach"));
  assert.equal(
    result.volumes.find((v) => v.key === "evidence-volume").remainingBytes,
    29 * GIB,
  );
});

test("does not allow callers to lower the minimum reserve", async (t) => {
  const fixture = await makeFixture(t);
  const result = preflightBuildStorage({
    ...validOptions(fixture),
    reserveBytes: 0,
  });
  assert.equal(result.ok, false);
  assert.ok(failureCodes(result).includes("invalid-reserve"));
});

test("uses the lowest capacity observation for a shared volume", async (t) => {
  const fixture = await makeFixture(t);
  let calls = 0;
  const result = preflightBuildStorage({
    ...validOptions(fixture),
    readCapacity: () => ({
      volumeKey: "shared",
      availableBytes: (++calls === 1 ? 40 : 34) * GIB,
    }),
  });
  assert.equal(result.ok, false);
  assert.ok(failureCodes(result).includes("estimated-output-breach"));
});
