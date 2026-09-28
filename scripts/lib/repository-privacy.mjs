import { createHash } from "node:crypto";
import { extname } from "node:path";

const blockedTrackedExtensions = new Set([
  ".aif",
  ".aiff",
  ".bak",
  ".db",
  ".flac",
  ".flm",
  ".flp",
  ".fst",
  ".key",
  ".log",
  ".m4a",
  ".mp3",
  ".ogg",
  ".p12",
  ".pem",
  ".pfx",
  ".sqlite",
  ".sqlite3",
  ".wav",
  ".wave",
]);

// Only these separately approved public parser fixtures bypass the FLP block.
// Both the exact repository path and SHA-256 must match.
const approvedFlpFixtureHashes = new Map([
  [
    "fixtures/parser-corpus/FIX-BASE-MIN.flp",
    "d3caba4e6e1a2cd47074b7b6e0f115d9d896e6da2e95705baff0777ef06b30ae",
  ],
  [
    "fixtures/parser-corpus/FIX-FL2024-A.flp",
    "02ed4f6fc4e3f471a7f0619ac31e64049eb34d9eaccc4b6bded15300b5644b72",
  ],
  [
    "fixtures/parser-corpus/FIX-FL2024-B.flp",
    "347aedcb7cc503a3f166bca2fa8e6c03099fc18de958e70a9cd2834c73a9fed7",
  ],
  [
    "fixtures/parser-corpus/FIX-FL2025-MIN.flp",
    "745f3cbb7ec095b8e03ab010ad6463c90e2c3616d4d671b868ef6fdbc9fe5fc5",
  ],
  [
    "fixtures/parser-corpus/FIX-FL2026-MIN.flp",
    "e0471d55032b7dfaa8bbcd496c487147149b9ce18e0b250882cc6f67f47480ee",
  ],
  [
    "fixtures/parser-corpus/FIX-FL2026-SAMPLE.flp",
    "dc11a613e34f2ec4918addf1ec2562d6a2c75ebc8c322f56bcb42d280e607b39",
  ],
  [
    "fixtures/parser-corpus/FIX-RB-TRUNC.flp",
    "38837f28cf0723434b9a089decc617af8e0069e19b12bd8d6d4cee612e59a9cd",
  ],
  [
    "fixtures/parser-corpus/FIX-RB-UNKNOWN.flp",
    "06c5b2c4cc6c2f2b647cb40bb745d2c2f5d92293aa36daab6e05b62d0ab0e9f6",
  ],
  [
    "fixtures/parser-corpus/FIX-RB-MALFORM.flp",
    "561fd951e783bcb111980f33ee287a5da928ea2f3770d97fced01823deedbb45",
  ],
  [
    "fixtures/parser-corpus/FIX-RB-LIMIT.flp",
    "950fb8dbd33b090d5f78c9b3c635a4cc316a3aa36d4fd1d44913674addefa743",
  ],
]);

// These older saves use the documented FLP event framing. The 2026 fixtures
// have newer opaque events, so only their raw path scan can be applied here.
const inspectableFlpFixtures = new Set([
  "fixtures/parser-corpus/FIX-BASE-MIN.flp",
  "fixtures/parser-corpus/FIX-FL2024-A.flp",
  "fixtures/parser-corpus/FIX-FL2024-B.flp",
  "fixtures/parser-corpus/FIX-FL2025-MIN.flp",
]);

const nonPersonalHomeNames = new Set([
  "artist",
  "example",
  "person",
  "producer",
  "public",
]);

const secretPatterns = [
  {
    name: "private-key",
    pattern: /-----BEGIN (?:[A-Z0-9]+ )?PRIVATE KEY-----/gu,
  },
  {
    name: "github-token",
    pattern:
      /\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})\b/gu,
  },
  {
    name: "npm-token",
    pattern: /\bnpm_[A-Za-z0-9]{30,}\b/gu,
  },
  {
    name: "aws-access-key",
    pattern: /\b(?:AKIA|ASIA)[A-Z0-9]{16}\b/gu,
  },
  {
    name: "google-api-key",
    pattern: /\bAIza[A-Za-z0-9_-]{35}\b/gu,
  },
  {
    name: "slack-token",
    pattern: /\bxox[baprs]-[A-Za-z0-9-]{20,}\b/gu,
  },
  {
    name: "assigned-secret",
    pattern:
      /\b(?:access[_-]?token|refresh[_-]?token|client[_-]?secret|api[_-]?key|password)\b\s*[=:]\s*["']?[A-Za-z0-9_./+=-]{20,}["']?/giu,
  },
];

const homePathPatterns = [
  /[A-Za-z]:[\\/]+(?:Users|Documents and Settings)[\\/]+([^\\/"'<>|:\r\n]+?)[\\/]+/giu,
  /\/(?:Users|home)\/([^/"'<>|:\r\n]+?)\//gu,
];

const lineAt = (text, index) => text.slice(0, index).split("\n").length;

export function inspectTrackedPath(path, sha256 = null) {
  const normalizedPath = path.replaceAll("\\", "/");
  const basename = normalizedPath.slice(normalizedPath.lastIndexOf("/") + 1);
  const extension = extname(basename).toLowerCase();
  const violations = [];

  if (extension === ".flp" && approvedFlpFixtureHashes.has(normalizedPath)) {
    if (
      sha256?.toLowerCase() !== approvedFlpFixtureHashes.get(normalizedPath)
    ) {
      violations.push({
        line: null,
        message: "approved fixture bytes differ from pinned SHA-256",
        path,
        rule: "fixture-hash-mismatch",
      });
    }
  } else if (blockedTrackedExtensions.has(extension)) {
    violations.push({
      line: null,
      message: `tracked private/generated ${extension} file`,
      path,
      rule: "blocked-extension",
    });
  }

  if (basename.startsWith(".env") && basename !== ".env.example") {
    violations.push({
      line: null,
      message: "tracked environment file",
      path,
      rule: "environment-file",
    });
  }

  return violations;
}

export function inspectTrackedText(path, text) {
  const violations = [];

  for (const { name, pattern } of secretPatterns) {
    pattern.lastIndex = 0;
    for (const match of text.matchAll(pattern)) {
      violations.push({
        line: lineAt(text, match.index ?? 0),
        message: "high-confidence credential material",
        path,
        rule: name,
      });
    }
  }

  for (const pattern of homePathPatterns) {
    pattern.lastIndex = 0;
    for (const match of text.matchAll(pattern)) {
      const homeName = match[1]?.trim().toLowerCase();
      if (homeName && !nonPersonalHomeNames.has(homeName)) {
        violations.push({
          line: lineAt(text, match.index ?? 0),
          message: "non-synthetic personal home path",
          path,
          rule: "personal-home-path",
        });
      }
    }
  }

  return violations;
}

export function inspectApprovedFixtureBinary(path, bytes) {
  const normalizedPath = path.replaceAll("\\", "/");
  if (!approvedFlpFixtureHashes.has(normalizedPath)) return [];

  const violations = [];
  const report = (rule, message) =>
    violations.push({ line: null, message, path, rule });

  // Search single-byte text and both UTF-16LE alignments in the binary stream.
  for (const text of [
    bytes.toString("latin1"),
    bytes.toString("utf16le"),
    bytes.subarray(1).toString("utf16le"),
  ]) {
    const homePath = /[A-Za-z]:[\\/]+Users[\\/]+([^\\/\x00:]+)[\\/]+/giu;
    for (const match of text.matchAll(homePath)) {
      if (match[1].toLowerCase() !== "public") {
        report(
          "fixture-personal-home-path",
          "non-Public home path in fixture bytes",
        );
        break;
      }
    }
    if (violations.some(({ rule }) => rule === "fixture-personal-home-path"))
      break;
  }

  if (!inspectableFlpFixtures.has(normalizedPath)) return violations;
  const malformed = () =>
    report("fixture-event-framing", "fixture event stream is not bounded");
  if (
    bytes.length < 22 ||
    bytes.toString("ascii", 0, 4) !== "FLhd" ||
    bytes.toString("ascii", 14, 18) !== "FLdt" ||
    bytes.readUInt32LE(18) !== bytes.length - 22
  ) {
    malformed();
    return violations;
  }

  let cursor = 22;
  while (cursor < bytes.length) {
    const id = bytes[cursor++];
    let size = 0;
    if (id < 64) size = 1;
    else if (id < 128) size = 2;
    else if (id < 192) size = 4;
    else {
      let shift = 0;
      while (true) {
        if (cursor >= bytes.length || shift > 28) {
          malformed();
          return violations;
        }
        const octet = bytes[cursor++];
        size += (octet & 0x7f) * 2 ** shift;
        if (octet < 0x80) break;
        shift += 7;
      }
    }
    if (size > bytes.length - cursor) {
      malformed();
      return violations;
    }
    if (id === 200) {
      report(
        "fixture-registration-metadata",
        "registration event in fixture bytes",
      );
    }
    cursor += size;
  }
  return violations;
}

export function inspectRepositoryEntries(entries) {
  return entries
    .flatMap(({ path, text, sha256, bytes }) => [
      ...inspectTrackedPath(path, sha256),
      ...(text === null ? [] : inspectTrackedText(path, text)),
      ...(bytes ? inspectApprovedFixtureBinary(path, bytes) : []),
    ])
    .sort(
      (left, right) =>
        left.path.localeCompare(right.path) ||
        (left.line ?? 0) - (right.line ?? 0) ||
        left.rule.localeCompare(right.rule),
    );
}

export function formatPrivacyViolation(violation) {
  const sensitivePathRule = new Set([
    "blocked-extension",
    "environment-file",
    "fixture-hash-mismatch",
  ]).has(violation.rule);
  const displayPath = sensitivePathRule
    ? `repository-path-sha256:${createHash("sha256")
        .update(violation.path)
        .digest("hex")
        .slice(0, 12)}`
    : violation.path;
  const location = violation.line
    ? `${displayPath}:${violation.line}`
    : displayPath;

  return `${location}: ${violation.message} [${violation.rule}]`;
}
