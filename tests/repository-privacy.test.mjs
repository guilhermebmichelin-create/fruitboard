import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  formatPrivacyViolation,
  inspectApprovedFixtureBinary,
  inspectRepositoryEntries,
  inspectTrackedPath,
  inspectTrackedText,
} from "../scripts/lib/repository-privacy.mjs";

const baselinePath = "fixtures/parser-corpus/FIX-BASE-MIN.flp";

function smallFlp(eventId) {
  const header = Buffer.alloc(22);
  header.write("FLhd", 0, "ascii");
  header.write("FLdt", 14, "ascii");
  header.writeUInt32LE(3, 18);
  return Buffer.concat([header, Buffer.from([eventId, 1, 0])]);
}

test("approved fixture binary scan catches registration events and private paths", () => {
  const registration = inspectApprovedFixtureBinary(
    baselinePath,
    smallFlp(200),
  );
  assert.equal(registration[0].rule, "fixture-registration-metadata");

  const pathBytes = Buffer.from("C:\\Users\\person\\Documents\\", "utf16le");
  const pathViolation = inspectApprovedFixtureBinary(
    baselinePath,
    Buffer.concat([Buffer.from([0]), pathBytes]),
  );
  assert.ok(
    pathViolation.some(({ rule }) => rule === "fixture-personal-home-path"),
  );
  assert.ok(pathViolation.every(({ message }) => !message.includes("person")));
  assert.ok(
    inspectApprovedFixtureBinary(
      baselinePath,
      Buffer.from("C:/Users/person/", "ascii"),
    ).some(({ rule }) => rule === "fixture-personal-home-path"),
  );
  assert.deepEqual(
    inspectApprovedFixtureBinary(
      "fixtures/parser-corpus/not-approved.flp",
      pathBytes,
    ),
    [],
  );
});

test("the committed parser corpus passes its binary privacy scan", () => {
  for (const name of [
    "FIX-BASE-MIN",
    "FIX-FL2024-A",
    "FIX-FL2024-B",
    "FIX-FL2025-MIN",
    "FIX-FL2026-MIN",
    "FIX-FL2026-SAMPLE",
    "FIX-FL2026-PATTERNS",
    "FIX-FL2026-3XOSC",
    "FIX-FL2026-MULTISAMPLER",
    "FIX-FL2026-NOTES",
    "FIX-FL2026-NAMED-EMPTY",
    "FIX-FL2026-MIXER-DEFAULT",
    "FIX-FL2026-MIXER-NAMES",
    "FIX-FL2026-MIXER-DUPLICATE",
    "FIX-FL2026-MIXER-SPECIAL",
    "FIX-RB-TRUNC",
    "FIX-RB-UNKNOWN",
    "FIX-RB-MALFORM",
    "FIX-RB-LIMIT",
  ]) {
    const path = `fixtures/parser-corpus/${name}.flp`;
    const bytes = readFileSync(new URL(`../${path}`, import.meta.url));
    assert.deepEqual(inspectApprovedFixtureBinary(path, bytes), [], name);
  }
});

test("note fixtures permit only their approved bytes and paths", () => {
  const registered = readFileSync(
    new URL(
      "../fixtures/parser-corpus/pattern-note-expectations.json",
      import.meta.url,
    ),
  );
  assert.equal(
    createHash("sha256").update(registered).digest("hex"),
    "6a7b161bc9d2d6240887a1ae04d5e0978bb4f9fcf072aa41e803a46b6f9a6295",
  );
  const expectations = JSON.parse(registered.toString("utf8"));
  for (const slot of ["F16", "F17"]) {
    const expected = expectations[slot];
    const path = `fixtures/parser-corpus/${expected.filename}`;
    const bytes = readFileSync(new URL(`../${path}`, import.meta.url));
    const hash = createHash("sha256").update(bytes).digest("hex");
    assert.equal(bytes.length, expected.bytes);
    assert.equal(hash, expected.sha256);
    assert.deepEqual(inspectTrackedPath(path, hash), []);
    const changed = Buffer.from(bytes);
    changed[changed.length - 1] ^= 1;
    assert.equal(
      inspectTrackedPath(
        path,
        createHash("sha256").update(changed).digest("hex"),
      )[0].rule,
      "fixture-hash-mismatch",
    );
    assert.equal(
      inspectTrackedPath(`other/${expected.filename}`, hash)[0].rule,
      "blocked-extension",
    );
  }
});

test("mixer fixtures permit only their approved bytes, paths and expectations", () => {
  const registered = readFileSync(
    new URL(
      "../fixtures/parser-corpus/mixer-insert-expectations.json",
      import.meta.url,
    ),
  );
  assert.equal(
    createHash("sha256").update(registered).digest("hex"),
    "cb89b238d9c7b836a6a5ddebaf044ae3f95559b0943364b4faa098fac1cb9d8e",
  );
  const expectations = JSON.parse(registered.toString("utf8"));
  assert.deepEqual(
    expectations.cases.map(({ slot }) => slot),
    ["F18", "F19", "F20", "F21"],
  );
  for (const expected of expectations.cases) {
    const path = `fixtures/parser-corpus/${expected.file}`;
    const bytes = readFileSync(new URL(`../${path}`, import.meta.url));
    const hash = createHash("sha256").update(bytes).digest("hex");
    assert.equal(bytes.length, expected.bytes);
    assert.equal(hash, expected.sha256);
    assert.deepEqual(inspectTrackedPath(path, hash), []);
    const changed = Buffer.from(bytes);
    changed[changed.length - 1] ^= 1;
    assert.equal(
      inspectTrackedPath(
        path,
        createHash("sha256").update(changed).digest("hex"),
      )[0].rule,
      "fixture-hash-mismatch",
    );
    assert.equal(
      inspectTrackedPath(`other/${expected.file}`, hash)[0].rule,
      "blocked-extension",
    );
  }
});

test("blocks tracked project, preset, audio, database, log, and key material", () => {
  for (const path of [
    "fixtures/song.flp",
    "fixtures/plugin.FST",
    "fixtures/master.wav",
    "fixtures/export.mp3",
    "local/fruitboard.db",
    "local/fruitboard.log",
    "signing/release.pfx",
    ".env.local",
  ]) {
    assert.notEqual(inspectTrackedPath(path).length, 0, path);
  }

  assert.deepEqual(inspectTrackedPath(".env.example"), []);
  assert.deepEqual(inspectTrackedPath("docs/review/interaction.mp4"), []);
});

test("blocks SQLite companions without exposing their repository paths", () => {
  for (const extension of ["db", "sqlite", "sqlite3"]) {
    for (const companion of ["wal", "shm", "journal"]) {
      for (const path of [
        `local/fruitboard.${extension}-${companion}`,
        `nested/local/project.backup.${extension}-${companion}`,
        `.private.${extension}-${companion}`,
        `local\\nested\\project.${extension.toUpperCase()}-${companion.toUpperCase()}`,
        `C:\\local\\nested\\project.${extension}-${companion}`,
      ]) {
        const [violation] = inspectTrackedPath(path);
        assert.equal(violation?.rule, "sqlite-companion", path);
        const output = formatPrivacyViolation(violation);
        assert.match(output, /repository-path-sha256:/);
        assert.doesNotMatch(output, /fruitboard|project|\.private|nested/);
      }
    }
  }
});

test("permits near matches to SQLite companion suffixes", () => {
  for (const path of [
    "docs/fruitboard.db-wal.md",
    "local/fruitboard.db-wal-backup",
    "local/fruitboard.db-wal.tmp",
    "local/fruitboard.sqlite4-shm",
    "local/fruitboard.sqlite3-journals",
    "local/fruitboard.db-wals",
    "local/fruitboarddb-wal",
    "local/fruitboard.dbwal",
    "local/db-wal",
    "nested/project.db-wal/readme.md",
    "nested\\project.sqlite-shm\\readme.md",
  ]) {
    assert.deepEqual(inspectTrackedPath(path), [], path);
  }
});

test("permits only the exact approved fixture bytes at their approved paths", () => {
  const path = "fixtures/parser-corpus/FIX-BASE-MIN.flp";
  const approvedHash =
    "d3caba4e6e1a2cd47074b7b6e0f115d9d896e6da2e95705baff0777ef06b30ae";

  assert.deepEqual(inspectTrackedPath(path, approvedHash), []);
  assert.deepEqual(
    inspectTrackedPath(path.replaceAll("/", "\\"), approvedHash),
    [],
  );
  assert.equal(inspectTrackedPath(path)[0].rule, "fixture-hash-mismatch");
  assert.equal(
    inspectTrackedPath(path, "0".repeat(64))[0].rule,
    "fixture-hash-mismatch",
  );
  assert.equal(
    inspectTrackedPath("fixtures/parser-corpus/other.flp", approvedHash)[0]
      .rule,
    "blocked-extension",
  );
  assert.equal(
    inspectTrackedPath("other/FIX-BASE-MIN.flp", approvedHash)[0].rule,
    "blocked-extension",
  );
  const [violation] = inspectRepositoryEntries([
    { path, text: null, sha256: "0".repeat(64) },
  ]);
  assert.equal(violation.rule, "fixture-hash-mismatch");
  assert.doesNotMatch(formatPrivacyViolation(violation), /FIX-BASE-MIN/);
});

test("rejects personal home paths without printing their sensitive suffix", () => {
  const windowsPath = [
    "C:",
    "Users",
    "real-account",
    "Private",
    "song.flp",
  ].join("\\");
  const unixPath = ["", "home", "real-account", "Private", "song.flp"].join(
    "/",
  );
  const violations = inspectTrackedText(
    "notes.txt",
    `${windowsPath}\n${unixPath}`,
  );

  assert.equal(violations.length, 2);
  assert.ok(violations.every(({ rule }) => rule === "personal-home-path"));
  assert.ok(
    violations.every(({ message }) => !message.includes("real-account")),
  );
});

test("permits synthetic and Public home paths", () => {
  const text = [
    String.raw`C:\Users\example\private.flp`,
    "/Users/producer/private.flp",
    "/home/artist/private.flp",
    "D:/Users/person/private.flp",
    String.raw`C:\Users\Public\Documents\fixture-silence.wav`,
  ].join("\n");

  assert.deepEqual(inspectTrackedText("safe-test.ts", text), []);
});

test("detects high-confidence credential formats without echoing values", () => {
  const samples = [
    ["github", ["ghp", "A".repeat(36)].join("_")],
    ["npm", ["npm", "b".repeat(36)].join("_")],
    ["aws", `AKIA${"C".repeat(16)}`],
    ["google", `AIza${"d".repeat(35)}`],
    ["private-key", `-----BEGIN ${"PRIVATE KEY"}-----`],
    ["assigned", `client_secret=${"e".repeat(24)}`],
  ];

  for (const [name, value] of samples) {
    const violations = inspectTrackedText(`${name}.txt`, value);
    assert.notEqual(violations.length, 0, name);
    assert.ok(violations.every(({ message }) => !message.includes(value)));
  }
});

test("reports repository entries in stable path and line order", () => {
  const privateKey = `-----BEGIN ${"PRIVATE KEY"}-----`;
  const violations = inspectRepositoryEntries([
    { path: "z/song.flp", text: null },
    { path: "a/credentials.txt", text: `safe\n${privateKey}` },
  ]);

  assert.deepEqual(
    violations.map(({ line, path, rule }) => ({ line, path, rule })),
    [
      { line: 2, path: "a/credentials.txt", rule: "private-key" },
      { line: null, path: "z/song.flp", rule: "blocked-extension" },
    ],
  );
});

test("privacy output fingerprints sensitive repository paths instead of echoing them", () => {
  const sensitivePath = "private-project/my unreleased song.flp";
  const [violation] = inspectTrackedPath(sensitivePath);
  const output = formatPrivacyViolation(violation);

  assert.doesNotMatch(output, /private-project|unreleased song/);
  assert.match(output, /^repository-path-sha256:[0-9a-f]{12}:/);
  assert.match(output, /\[blocked-extension\]$/);
});
