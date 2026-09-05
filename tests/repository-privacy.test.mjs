import assert from "node:assert/strict";
import test from "node:test";
import {
  formatPrivacyViolation,
  inspectRepositoryEntries,
  inspectTrackedPath,
  inspectTrackedText,
} from "../scripts/lib/repository-privacy.mjs";

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

test("permits the explicit synthetic homes used by redaction regressions", () => {
  const text = [
    String.raw`C:\Users\example\private.flp`,
    "/Users/producer/private.flp",
    "/home/artist/private.flp",
    "D:/Users/person/private.flp",
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
