import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const readRootFile = (path) =>
  readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

const PRODUCTION_SOURCES = [
  "crates/filesystem-enumeration/src/lib.rs",
  "crates/scan-execution/src/lib.rs",
  "crates/scan-execution/src/followups.rs",
  "crates/filesystem-watcher/src/lib.rs",
  "crates/filesystem-watcher/src/coalescer.rs",
  "crates/filesystem-watcher/src/path.rs",
  "crates/filesystem-watcher/src/platform.rs",
];

const TEST_SOURCES_WITH_FIXTURE_IO = [
  "crates/filesystem-enumeration/src/tests.rs",
  "crates/scan-execution/src/tests.rs",
  "crates/filesystem-watcher/src/tests.rs",
  "crates/filesystem-watcher/src/tests_windows.rs",
];

test("dependency manifests contain no PyFLP dependency", () => {
  const manifests = [
    "pyproject.toml",
    "uv.lock",
    "Cargo.toml",
    "Cargo.lock",
    "package.json",
    "pnpm-lock.yaml",
    "apps/client/package.json",
    "apps/desktop/package.json",
    "packages/ui/package.json",
    "crates/filesystem-enumeration/Cargo.toml",
    "crates/scan-execution/Cargo.toml",
    "crates/filesystem-watcher/Cargo.toml",
    "crates/reconciliation/Cargo.toml",
    "crates/storage-sqlite/Cargo.toml",
  ];

  for (const manifest of manifests) {
    const content = readRootFile(manifest);
    assert.doesNotMatch(content, /pyflp/i, `${manifest} must not reference PyFLP`);
  }
});

test("discovery sources perform zero file-content reads", () => {
  // Windows content-read entry points must never appear in discovery code.
  // Handle-bound metadata + directory listing (CreateFileW, NtCreateFile,
  // NtQueryDirectoryFile, GetFileInformationByHandleEx,
  // ReadDirectoryChangesW) are the only allowed handle uses; they observe
  // metadata without reading file bytes.
  const forbiddenContentReads = [
    /NtReadFile/,
    /ReadFile/,
    /parse_flp/,
    /pyflp/i,
    /CfHydrate/i,
    /HydratePlaceholder/,
  ];

  for (const source of PRODUCTION_SOURCES) {
    const content = readRootFile(source);
    for (const pattern of forbiddenContentReads) {
      assert.doesNotMatch(content, pattern, `${source} must not contain ${pattern}`);
    }
    // No standard-library file-content reads in production discovery code.
    // Fixture byte checks live only in the test sources below.
    assert.doesNotMatch(
      content,
      /std::fs::read|std::fs::read_to_string|tokio::fs/,
      `${source} must not read file contents`,
    );
    assert.doesNotMatch(
      content,
      /std::fs::write/,
      `${source} must never mutate sources`,
    );
  }
});

test("enumeration uses only handle-bound metadata APIs", () => {
  const enumeration = readRootFile("crates/filesystem-enumeration/src/lib.rs");

  assert.match(enumeration, /CreateFileW/, "metadata opens stay handle-bound");
  assert.match(enumeration, /NtCreateFile/, "metadata opens stay handle-bound");
  assert.match(enumeration, /NtQueryDirectoryFile/, "directory listing only");
  assert.doesNotMatch(enumeration, /NtReadFile/, "no content reads");
  assert.doesNotMatch(enumeration, /ReadFile/, "no content reads");
});

test("source-byte preservation fixtures hash before and after", () => {
  // The authoritative scan and enumeration fixtures snapshot marker bytes
  // before discovery and assert byte equality afterwards: discovery never
  // reads or writes source bytes beyond handle-bound metadata.
  const enumerationTests = readRootFile("crates/filesystem-enumeration/src/tests.rs");
  assert.match(
    enumerationTests,
    /read marker before scan/,
    "enumeration must snapshot fixture bytes before",
  );
  assert.match(
    enumerationTests,
    /read marker after scan/,
    "enumeration must re-read fixture bytes after",
  );
  assert.match(
    enumerationTests,
    /assert_eq!\(before, after\)/,
    "enumeration must assert byte equality",
  );

  const executionTests = readRootFile("crates/scan-execution/src/tests.rs");
  assert.match(
    executionTests,
    /marker_before/,
    "worker must snapshot fixture bytes before",
  );
  assert.match(
    executionTests,
    /marker_after/,
    "worker must re-read fixture bytes after",
  );
  assert.match(
    executionTests,
    /source bytes are never read or written/,
    "worker must assert source-byte preservation",
  );

  for (const source of TEST_SOURCES_WITH_FIXTURE_IO) {
    assert.ok(readRootFile(source).length > 0, `${source} must exist`);
  }
});
