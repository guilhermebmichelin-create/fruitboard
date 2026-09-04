import { readFileSync } from "node:fs";

const SEMVER_PATTERN = /(?:^|[^0-9])(\d+)\.(\d+)\.(\d+)(?:[^0-9]|$)/;

export function parseVersion(value, label = "version") {
  const match = String(value).match(SEMVER_PATTERN);
  if (!match) {
    throw new TypeError(`${label} must contain a numeric x.y.z version`);
  }

  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    normalized: `${Number(match[1])}.${Number(match[2])}.${Number(match[3])}`,
  };
}

export function compareVersions(left, right) {
  const a = parseVersion(left, "left version");
  const b = parseVersion(right, "right version");

  for (const key of ["major", "minor", "patch"]) {
    if (a[key] !== b[key]) {
      return a[key] < b[key] ? -1 : 1;
    }
  }

  return 0;
}

export function evaluateSqliteWalSafety(candidate, sqlitePolicy) {
  const version = parseVersion(candidate, "SQLite version").normalized;
  const minimum = parseVersion(
    sqlitePolicy.minimumWalSafeVersion,
    "minimum WAL-safe SQLite version",
  ).normalized;
  const backports = sqlitePolicy.approvedFixedBackports.map(
    (item) => parseVersion(item, "approved SQLite backport").normalized,
  );

  if (compareVersions(version, minimum) >= 0) {
    return {
      safe: true,
      version,
      reason: `SQLite ${version} meets the ${minimum} minimum`,
    };
  }

  if (backports.includes(version)) {
    return {
      safe: true,
      version,
      reason: `SQLite ${version} is an explicitly approved fixed backport`,
    };
  }

  return {
    safe: false,
    version,
    reason: `SQLite ${version} is below ${minimum} and is not an approved backport`,
  };
}

export function readToolchainPolicy(
  policyUrl = new URL("../../tools/toolchain-policy.json", import.meta.url),
) {
  const policy = JSON.parse(readFileSync(policyUrl, "utf8"));
  const requiredStrings = ["node", "pnpm", "corepack", "rust", "python", "uv"];

  if (policy.schemaVersion !== 1) {
    throw new TypeError("toolchain policy schemaVersion must be 1");
  }

  for (const key of requiredStrings) {
    parseVersion(policy[key], `policy.${key}`);
  }

  if (
    !Array.isArray(policy.rustComponents) ||
    policy.rustComponents.length === 0
  ) {
    throw new TypeError("policy.rustComponents must be a non-empty array");
  }

  if (!policy.windowsRustTarget) {
    throw new TypeError("policy.windowsRustTarget is required");
  }

  evaluateSqliteWalSafety(policy.sqlite.minimumWalSafeVersion, policy.sqlite);
  return policy;
}
