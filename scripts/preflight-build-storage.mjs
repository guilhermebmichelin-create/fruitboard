#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

export const GIB = 1024 ** 3;
export const MIN_FREE_RESERVE_BYTES = 30 * GIB;
export const DEFAULT_MAX_CACHE_ENTRIES = 100_000;

const LOCATION_NAMES = ["source", "build", "cache", "evidence"];
const WRITABLE_LOCATIONS = new Set(["build", "cache"]);
const PATH_OPTIONS = Object.freeze({
  "--source": "sourcePath",
  "--source-root": "sourcePath",
  "--build": "buildPath",
  "--build-root": "buildPath",
  "--cache": "cachePath",
  "--cache-root": "cachePath",
  "--evidence": "evidencePath",
  "--evidence-root": "evidencePath",
});
const realpath = fs.realpathSync.native ?? fs.realpathSync;

const fail = (code, message, details = {}) => ({
  code,
  message,
  ...details,
});

function comparisonPath(value) {
  const normalized = path.normalize(value);
  return process.platform === "win32" ? normalized.toLowerCase() : normalized;
}

export function isSameOrDescendant(candidate, parent) {
  const relative = path.relative(
    comparisonPath(parent),
    comparisonPath(candidate),
  );
  return (
    relative === "" ||
    (!relative.startsWith(`..${path.sep}`) &&
      relative !== ".." &&
      !path.isAbsolute(relative))
  );
}

function overlaps(left, right) {
  return isSameOrDescendant(left, right) || isSameOrDescendant(right, left);
}

function isReparsePoint(stats) {
  return typeof stats.isSymbolicLink === "function" && stats.isSymbolicLink();
}

function inspectPathComponents(absolutePath) {
  const root = path.parse(absolutePath).root;
  const relative = path.relative(root, absolutePath);
  const components = relative === "" ? [] : relative.split(path.sep);
  const points = [];
  let current = root;

  for (const component of components) {
    current = path.join(current, component);
    const stats = fs.lstatSync(current);
    if (isReparsePoint(stats)) {
      points.push({ path: current, resolvedPath: realpath(current) });
    }
  }
  return points;
}

export function resolveLocation(role, inputPath) {
  if (typeof inputPath !== "string" || inputPath.trim() === "") {
    throw new TypeError(`${role} location must be a non-empty path`);
  }

  const requestedPath = path.resolve(inputPath);
  const reparsePoints = inspectPathComponents(requestedPath);
  const resolvedPath = realpath(requestedPath);
  const stats = fs.statSync(resolvedPath);
  if (!stats.isDirectory()) {
    throw new TypeError(`${role} location must be an existing directory`);
  }

  return {
    role,
    requestedPath,
    resolvedPath,
    reparsePoints,
    device: Number.isSafeInteger(stats.dev) ? stats.dev : null,
  };
}

function defaultVolumeKey(locationPath, device = null) {
  const root = path.parse(locationPath).root || locationPath;
  return `root:${comparisonPath(root)}|device:${device ?? "unknown"}`;
}

export function readVolumeCapacity(locationPath) {
  const stats = fs.statfsSync(locationPath);
  const blockSize = Number(stats.frsize || stats.bsize);
  const availableBlocks = Number(stats.bavail ?? stats.bfree);
  if (
    !Number.isSafeInteger(blockSize) ||
    blockSize <= 0 ||
    !Number.isSafeInteger(availableBlocks) ||
    availableBlocks < 0
  ) {
    throw new Error("filesystem capacity is not safely representable");
  }

  const availableBytes = blockSize * availableBlocks;
  if (!Number.isSafeInteger(availableBytes)) {
    throw new Error("filesystem capacity exceeds the safe integer range");
  }
  let device = null;
  try {
    const locationStats = fs.statSync(locationPath);
    device = Number.isSafeInteger(locationStats.dev) ? locationStats.dev : null;
  } catch {
    // The statfs result is still usable; a missing device only weakens dedupe.
  }
  return {
    availableBytes,
    volumeKey: defaultVolumeKey(locationPath, device),
    displayRoot: path.parse(locationPath).root || locationPath,
  };
}

function parseBytes(value, label) {
  let parsed;
  if (typeof value === "bigint") {
    parsed =
      value >= 0n && value <= BigInt(Number.MAX_SAFE_INTEGER)
        ? Number(value)
        : NaN;
  } else if (typeof value === "number") {
    parsed = value;
  } else if (typeof value === "string" && /^\d+$/u.test(value)) {
    parsed = Number(value);
  }
  if (!Number.isSafeInteger(parsed) || parsed < 0) {
    throw new TypeError(`${label} must be a non-negative safe integer`);
  }
  return parsed;
}

function parseGiB(value, label) {
  const text = String(value).trim();
  const parsed = Number(text);
  if (text === "" || !Number.isFinite(parsed) || parsed < 0) {
    throw new TypeError(`${label} must be a non-negative number`);
  }
  const bytes = Math.ceil(parsed * GIB);
  if (!Number.isSafeInteger(bytes)) {
    throw new TypeError(`${label} is too large`);
  }
  return bytes;
}

function selectedPath(options, role) {
  return options[`${role}Path`] ?? options[role];
}

function validateRelationships(locations) {
  const failures = [];
  for (const [left, right] of [
    ["source", "evidence"],
    ["build", "evidence"],
    ["cache", "evidence"],
  ]) {
    if (overlaps(locations[left].resolvedPath, locations[right].resolvedPath)) {
      failures.push(
        fail(
          "protected-path-overlap",
          `${left} and ${right} locations overlap after path resolution`,
          { left, right },
        ),
      );
    }
  }

  if (
    isSameOrDescendant(
      locations.source.resolvedPath,
      locations.cache.resolvedPath,
    )
  ) {
    failures.push(
      fail(
        "cache-contains-source",
        "writable cache contains the protected source location",
        { left: "cache", right: "source" },
      ),
    );
  }
  if (
    isSameOrDescendant(
      locations.source.resolvedPath,
      locations.build.resolvedPath,
    ) &&
    !isSameOrDescendant(
      locations.build.resolvedPath,
      locations.source.resolvedPath,
    )
  ) {
    failures.push(
      fail(
        "build-contains-source",
        "build location contains the protected source location",
        { left: "build", right: "source" },
      ),
    );
  }
  return failures;
}

function normalizeCapacity(raw, location) {
  const availableBytes =
    typeof raw === "number" ? raw : (raw?.availableBytes ?? raw?.freeBytes);
  return {
    availableBytes: parseBytes(availableBytes, `${location.role} capacity`),
    volumeKey:
      typeof raw === "object" && raw?.volumeKey
        ? String(raw.volumeKey)
        : defaultVolumeKey(location.resolvedPath, location.device),
    displayRoot:
      typeof raw === "object" && raw?.displayRoot
        ? String(raw.displayRoot)
        : path.parse(location.resolvedPath).root || location.resolvedPath,
  };
}

function inspectVolumes(locations, estimateBytes, reserveBytes, readCapacity) {
  const failures = [];
  const observations = new Map();
  for (const location of Object.values(locations)) {
    try {
      const capacity = normalizeCapacity(
        readCapacity(location.resolvedPath, location),
        location,
      );
      const entries = observations.get(capacity.volumeKey) ?? [];
      entries.push({ location, capacity });
      observations.set(capacity.volumeKey, entries);
    } catch (error) {
      failures.push(
        fail(
          "capacity-unavailable",
          `${location.role} volume capacity could not be established: ${error.message}`,
          { role: location.role },
        ),
      );
    }
  }

  const volumes = [];
  for (const [key, entries] of observations) {
    const first = entries[0].capacity;
    const roles = entries.map(({ location }) => location.role);
    const estimatedAdditionalOutputBytes = roles.some((role) =>
      WRITABLE_LOCATIONS.has(role),
    )
      ? estimateBytes
      : 0;
    const remainingBytes =
      first.availableBytes - estimatedAdditionalOutputBytes;
    const alreadyBelowReserve = first.availableBytes < reserveBytes;
    const outputBreachesReserve = remainingBytes < reserveBytes;
    volumes.push({
      key,
      displayRoot: first.displayRoot,
      roles,
      availableBytes: first.availableBytes,
      estimatedAdditionalOutputBytes,
      remainingBytes,
      reserveBytes,
      passesReserve: !alreadyBelowReserve && !outputBreachesReserve,
    });

    if (alreadyBelowReserve) {
      failures.push(
        fail(
          "reserve-breach",
          `${roles.join("/")} volume is already below the ${formatBytes(reserveBytes)} reserve`,
          { roles, volumeKey: key },
        ),
      );
    } else if (outputBreachesReserve) {
      failures.push(
        fail(
          "estimated-output-breach",
          `${roles.join("/")} volume would fall below the ${formatBytes(reserveBytes)} reserve after estimated output`,
          { roles, volumeKey: key },
        ),
      );
    }
  }
  return { failures, volumes };
}

function scanCache(cache, protectedLocations, maxEntries) {
  const result = {
    status: "isolated",
    entriesScanned: 0,
    regularFilesScanned: 0,
    hardlinkedFiles: [],
    reparsePoints: [],
    uncertainty: [],
  };
  const failures = [];
  const root = cache.resolvedPath;
  const directories = [root];
  const visitedDirectories = new Set([comparisonPath(root)]);
  const identities = new Map();
  let uncertain = false;
  let unsafe = false;

  const markUncertain = (code, message, details = {}) => {
    uncertain = true;
    result.uncertainty.push({ code, ...details });
    failures.push(fail(code, message, details));
  };
  const relativePath = (value) => path.relative(root, value) || ".";

  const inspectFile = (filePath, displayPath) => {
    let stats;
    try {
      stats = fs.statSync(filePath);
    } catch (error) {
      markUncertain(
        "cache-scan-uncertain",
        `cache file metadata could not be read: ${error.message}`,
        { relativePath: relativePath(displayPath) },
      );
      return;
    }
    if (!stats.isFile()) {
      markUncertain(
        "cache-scan-uncertain",
        "cache entry changed to a non-file during inspection",
        { relativePath: relativePath(displayPath) },
      );
      return;
    }

    result.regularFilesScanned += 1;
    const relative = relativePath(displayPath);
    if (!Number.isSafeInteger(stats.nlink) || stats.nlink < 1) {
      markUncertain(
        "hardlink-inspection-uncertain",
        "filesystem did not provide a trustworthy hardlink count",
        { relativePath: relative },
      );
    } else if (stats.nlink > 1) {
      unsafe = true;
      result.hardlinkedFiles.push({
        relativePath: relative,
        linkCount: stats.nlink,
      });
      failures.push(
        fail(
          "hardlink-alias",
          "writable cache contains a file with hardlink aliases; it is not isolated",
          { relativePath: relative, linkCount: stats.nlink },
        ),
      );
    }

    if (Number.isSafeInteger(stats.dev) && Number.isSafeInteger(stats.ino)) {
      const identity = `${stats.dev}:${stats.ino}`;
      const previous = identities.get(identity);
      if (previous && previous !== relative) {
        unsafe = true;
        failures.push(
          fail(
            "hardlink-alias",
            "two cache paths resolve to the same file identity",
            { relativePath: relative, otherRelativePath: previous },
          ),
        );
      } else {
        identities.set(identity, relative);
      }
    }
  };

  while (directories.length > 0 && !uncertain) {
    const directory = directories.pop();
    let entries;
    try {
      entries = fs.readdirSync(directory, { withFileTypes: true });
    } catch (error) {
      markUncertain(
        "cache-scan-uncertain",
        `writable cache could not be fully enumerated: ${error.message}`,
        { relativePath: relativePath(directory) },
      );
      break;
    }

    for (const entry of entries) {
      if (result.entriesScanned >= maxEntries) {
        markUncertain(
          "cache-scan-truncated",
          `writable cache inspection reached the ${maxEntries}-entry bound`,
        );
        break;
      }
      result.entriesScanned += 1;
      const candidate = path.join(directory, entry.name);
      let stats;
      try {
        stats = fs.lstatSync(candidate);
      } catch (error) {
        markUncertain(
          "cache-scan-uncertain",
          `cache entry metadata could not be read: ${error.message}`,
          { relativePath: relativePath(candidate) },
        );
        break;
      }

      if (isReparsePoint(stats)) {
        let target;
        try {
          target = realpath(candidate);
        } catch (error) {
          markUncertain(
            "cache-reparse-uncertain",
            `cache reparse point could not be resolved: ${error.message}`,
            { relativePath: relativePath(candidate) },
          );
          break;
        }
        result.reparsePoints.push({
          relativePath: relativePath(candidate),
          resolvedPath: target,
        });
        if (!isSameOrDescendant(target, root)) {
          const protectedTarget = protectedLocations.some((location) =>
            overlaps(target, location.resolvedPath),
          );
          markUncertain(
            protectedTarget
              ? "cache-reparse-protected-overlap"
              : "cache-reparse-uncertain",
            protectedTarget
              ? "cache reparse point resolves into a protected source or evidence location"
              : "cache reparse point resolves outside the selected cache; isolation is unverified",
            { relativePath: relativePath(candidate) },
          );
          break;
        }
        let targetStats;
        try {
          targetStats = fs.statSync(target);
        } catch (error) {
          markUncertain(
            "cache-reparse-uncertain",
            `cache reparse target metadata could not be read: ${error.message}`,
            { relativePath: relativePath(candidate) },
          );
          break;
        }
        if (targetStats.isDirectory()) {
          const key = comparisonPath(target);
          if (!visitedDirectories.has(key)) {
            visitedDirectories.add(key);
            directories.push(target);
          }
        } else if (targetStats.isFile()) {
          inspectFile(target, candidate);
        } else {
          markUncertain(
            "cache-scan-uncertain",
            "cache reparse point resolves to an unsupported filesystem object",
            { relativePath: relativePath(candidate) },
          );
          break;
        }
      } else if (stats.isDirectory()) {
        const key = comparisonPath(candidate);
        if (!visitedDirectories.has(key)) {
          visitedDirectories.add(key);
          directories.push(candidate);
        }
      } else if (stats.isFile()) {
        inspectFile(candidate, candidate);
      } else {
        markUncertain(
          "cache-scan-uncertain",
          "writable cache contains an unsupported filesystem object",
          { relativePath: relativePath(candidate) },
        );
        break;
      }
    }
  }

  result.status = uncertain ? "uncertain" : unsafe ? "unsafe" : "isolated";
  return { result, failures };
}

function emptyResult() {
  return {
    ok: false,
    reserveBytes: MIN_FREE_RESERVE_BYTES,
    estimatedAdditionalOutputBytes: null,
    locations: {},
    volumes: [],
    cache: {
      status: "not-checked",
      entriesScanned: 0,
      regularFilesScanned: 0,
      hardlinkedFiles: [],
      reparsePoints: [],
      uncertainty: [],
    },
    failures: [],
    warnings: [],
  };
}

export function preflightBuildStorage(options = {}) {
  const result = emptyResult();
  try {
    result.reserveBytes = parseBytes(
      options.reserveBytes ?? MIN_FREE_RESERVE_BYTES,
      "reserveBytes",
    );
  } catch (error) {
    result.failures.push(fail("invalid-reserve", error.message));
    return result;
  }

  const estimateValue =
    options.estimatedAdditionalOutputBytes ?? options.estimatedOutputBytes;
  if (estimateValue === undefined) {
    result.failures.push(
      fail(
        "missing-estimate",
        "estimated additional output is required; an unbounded build is not certified",
      ),
    );
    return result;
  }
  try {
    result.estimatedAdditionalOutputBytes = parseBytes(
      estimateValue,
      "estimatedAdditionalOutputBytes",
    );
  } catch (error) {
    result.failures.push(fail("invalid-estimate", error.message));
    return result;
  }

  const maxEntries = options.maxCacheEntries ?? DEFAULT_MAX_CACHE_ENTRIES;
  if (!Number.isSafeInteger(maxEntries) || maxEntries < 1) {
    result.failures.push(
      fail(
        "invalid-cache-limit",
        "maxCacheEntries must be a positive safe integer",
      ),
    );
    return result;
  }

  const locations = {};
  for (const role of LOCATION_NAMES) {
    try {
      locations[role] = resolveLocation(role, selectedPath(options, role));
    } catch (error) {
      result.failures.push(fail("invalid-location", error.message, { role }));
    }
  }
  if (result.failures.length > 0) {
    return result;
  }
  result.locations = locations;
  for (const location of Object.values(locations)) {
    if (location.reparsePoints.length > 0) {
      result.warnings.push({
        code: "resolved-reparse-point",
        role: location.role,
        count: location.reparsePoints.length,
      });
    }
  }

  const relationshipFailures = validateRelationships(locations);
  result.failures.push(...relationshipFailures);
  const readCapacity =
    options.readCapacity ??
    options.capacityReader ??
    ((resolvedPath) => readVolumeCapacity(resolvedPath));
  const volumes = inspectVolumes(
    locations,
    result.estimatedAdditionalOutputBytes,
    result.reserveBytes,
    readCapacity,
  );
  result.volumes = volumes.volumes;
  result.failures.push(...volumes.failures);

  if (relationshipFailures.length === 0) {
    const scan = scanCache(
      locations.cache,
      [locations.source, locations.evidence],
      maxEntries,
    );
    result.cache = scan.result;
    result.failures.push(...scan.failures);
  }
  result.ok =
    result.failures.length === 0 && result.cache.status === "isolated";
  return result;
}

export function formatBytes(bytes) {
  return `${(bytes / GIB).toFixed(2)} GiB`;
}

export function formatPreflightReport(result) {
  const lines = [
    `Build storage preflight ${result.ok ? "passed" : "failed"}.`,
    `- reserve: ${formatBytes(result.reserveBytes)}`,
    `- estimated additional output: ${formatBytes(result.estimatedAdditionalOutputBytes ?? 0)}`,
  ];
  for (const location of Object.values(result.locations)) {
    const note =
      location.reparsePoints.length === 0
        ? ""
        : `; resolved ${location.reparsePoints.length} reparse point(s)`;
    lines.push(`- ${location.role}: ${location.resolvedPath}${note}`);
  }
  for (const volume of result.volumes) {
    lines.push(
      `- volume ${volume.displayRoot} (${volume.roles.join(", ")}): available ${formatBytes(volume.availableBytes)}; estimated output ${formatBytes(volume.estimatedAdditionalOutputBytes)}; remaining ${formatBytes(volume.remainingBytes)}; reserve ${formatBytes(volume.reserveBytes)}`,
    );
  }
  lines.push(
    `- cache hardlink inspection: ${result.cache.status}; ${result.cache.regularFilesScanned} regular file(s) scanned across ${result.cache.entriesScanned} entr${result.cache.entriesScanned === 1 ? "y" : "ies"}`,
  );
  for (const warning of result.warnings) {
    lines.push(
      `- warning [${warning.code}]: ${warning.role} had ${warning.count} resolved reparse point(s); unrelated reparse points outside the selected cache were not scanned`,
    );
  }
  for (const item of result.failures) {
    lines.push(`- [${item.code}] ${item.message}`);
  }
  return lines.join("\n");
}

function nextArgument(argv, index, option) {
  const value = argv[index + 1];
  if (value === undefined || value.startsWith("--")) {
    throw new TypeError(`${option} requires a value`);
  }
  return value;
}

export function parseCliArgs(argv) {
  const options = { json: false };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--help" || argument === "-h") {
      return { help: true };
    }
    if (argument === "--json") {
      options.json = true;
      continue;
    }
    if (PATH_OPTIONS[argument]) {
      options[PATH_OPTIONS[argument]] = nextArgument(argv, index, argument);
      index += 1;
      continue;
    }
    if (argument === "--estimated-output-bytes") {
      options.estimatedAdditionalOutputBytes = parseBytes(
        nextArgument(argv, index, argument),
        argument,
      );
      index += 1;
      continue;
    }
    if (argument === "--estimated-output-gib") {
      options.estimatedAdditionalOutputBytes = parseGiB(
        nextArgument(argv, index, argument),
        argument,
      );
      index += 1;
      continue;
    }
    if (argument === "--max-cache-entries") {
      const value = Number.parseInt(nextArgument(argv, index, argument), 10);
      if (!Number.isSafeInteger(value) || value < 1) {
        throw new TypeError(`${argument} must be a positive safe integer`);
      }
      options.maxCacheEntries = value;
      index += 1;
      continue;
    }
    throw new TypeError(`unknown option: ${argument}`);
  }

  for (const role of LOCATION_NAMES) {
    if (options[`${role}Path`] === undefined) {
      throw new TypeError(`--${role} is required`);
    }
  }
  if (options.estimatedAdditionalOutputBytes === undefined) {
    throw new TypeError(
      "--estimated-output-gib or --estimated-output-bytes is required",
    );
  }
  return options;
}

export function usage() {
  return [
    "Usage:",
    "  node scripts/preflight-build-storage.mjs --source <dir> --build <dir>",
    "    --cache <dir> --evidence <dir> --estimated-output-gib <number>",
    "",
    "The preflight is read-only. It reports capacity for every relevant volume,",
    "requires the 30 GiB reserve after estimated output, and scans only the",
    "explicitly selected cache for hardlink aliases.",
  ].join("\n");
}

export function main(argv = process.argv.slice(2)) {
  try {
    const options = parseCliArgs(argv);
    if (options.help) {
      console.log(usage());
      return 0;
    }
    const result = preflightBuildStorage(options);
    console.log(
      options.json
        ? JSON.stringify(result, null, 2)
        : formatPreflightReport(result),
    );
    return result.ok ? 0 : 1;
  } catch (error) {
    console.error(`${error.message}\n\n${usage()}`);
    return 2;
  }
}

if (
  process.argv[1] !== undefined &&
  import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href
) {
  process.exitCode = main();
}
