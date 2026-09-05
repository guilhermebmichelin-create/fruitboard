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

const syntheticHomeNames = new Set(["artist", "example", "person", "producer"]);

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

export function inspectTrackedPath(path) {
  const normalizedPath = path.replaceAll("\\", "/");
  const basename = normalizedPath.slice(normalizedPath.lastIndexOf("/") + 1);
  const extension = extname(basename).toLowerCase();
  const violations = [];

  if (blockedTrackedExtensions.has(extension)) {
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
      if (homeName && !syntheticHomeNames.has(homeName)) {
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

export function inspectRepositoryEntries(entries) {
  return entries
    .flatMap(({ path, text }) => [
      ...inspectTrackedPath(path),
      ...(text === null ? [] : inspectTrackedText(path, text)),
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
