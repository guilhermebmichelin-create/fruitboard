import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const args = process.argv.slice(2);
const usage =
  "usage: node installed-ntfs-cases.mjs <journey-root> <app-path> <lock-path> <run-id>";

if (args.length !== 4 || args.some((value) => value.trim() === "")) {
  console.error(usage);
  process.exit(2);
}

const [journeyArgument, appArgument, lockArgument, runId] = args;
const journeyRoot = path.resolve(journeyArgument);
const appPath = path.resolve(appArgument);
const lockPath = path.resolve(lockArgument);
const dataDirectory = path.resolve(
  process.env.FRUITBOARD_FOUNDATION_SMOKE_DATA_DIRECTORY ?? "",
);
const storageDirectory = path.join(dataDirectory, "storage");
const repositoryRoot = path.resolve(
  process.env.FRUITBOARD_REPOSITORY_ROOT ?? "",
);
const expectedLockPid = Number.parseInt(
  process.env.FRUITBOARD_FOUNDATION_SMOKE_LOCK_PID ?? "",
  10,
);
const transcriptPath = path.join(journeyRoot, "ntfs-cases.jsonl");
const marker =
  "FRUITBOARD SYNTHETIC FIXTURE. NOT AN FL STUDIO PROJECT. NO PRIVATE DATA.\n";

function relativeArtifact(candidate) {
  const relative = path.relative(journeyRoot, candidate);
  if (relative === "") return ".";
  return relative.split(path.sep).join("/");
}

function safeError(error) {
  return String(error?.message ?? error)
    .replaceAll("\r", " ")
    .replaceAll("\n", " ");
}

function hashFile(filePath) {
  return createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
}

function isProcessAlive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try {
    const output = execFileSync(
      "tasklist.exe",
      ["/FI", `PID eq ${pid}`, "/FO", "CSV", "/NH"],
      {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
        windowsHide: true,
      },
    );
    return output.includes(`"${pid}"`);
  } catch {
    return false;
  }
}

function readLockOwner() {
  const raw = fs.readFileSync(lockPath, "utf8").replace(/^\uFEFF/, "");
  return JSON.parse(raw);
}

function assertLockOwnership() {
  if (!Number.isInteger(expectedLockPid) || expectedLockPid <= 0) {
    throw new Error(
      "the orchestration wrapper did not provide its lock-owner PID",
    );
  }
  let owner;
  try {
    owner = readLockOwner();
  } catch {
    throw new Error(
      "the Foundation Smoke lock could not be read before a shared-data operation",
    );
  }
  if (
    owner?.schemaVersion !== 1 ||
    owner.runId !== runId ||
    owner.pid !== expectedLockPid ||
    owner.purpose !== "installed-ntfs-cases-20260912" ||
    path.resolve(owner.runRoot ?? "").toLowerCase() !==
      journeyRoot.toLowerCase() ||
    path.resolve(owner.installDirectory ?? "").toLowerCase() !==
      path.dirname(appPath).toLowerCase()
  ) {
    throw new Error("the Foundation Smoke lock is not owned by this run");
  }
  if (!isProcessAlive(expectedLockPid)) {
    throw new Error("the Foundation Smoke lock owner is no longer alive");
  }
  return owner;
}

function log(entry) {
  const line = JSON.stringify({ t: new Date().toISOString(), ...entry });
  fs.appendFileSync(transcriptPath, `${line}\n`, "utf8");
  console.log(line);
}

function sleep(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function getFreePort() {
  return new Promise((resolve, reject) => {
    import("node:net").then(({ default: net }) => {
      const server = net.createServer();
      server.listen(0, "127.0.0.1", () => {
        const port = server.address().port;
        server.close(() => resolve(port));
      });
      server.on("error", reject);
    });
  });
}

async function discover(port, timeoutMilliseconds = 60000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMilliseconds) {
    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), 4000);
      let targets;
      try {
        const response = await fetch(`http://127.0.0.1:${port}/json/list`, {
          signal: controller.signal,
        });
        targets = await response.json();
      } finally {
        clearTimeout(timer);
      }
      for (const target of targets ?? []) {
        if (
          target.type !== "page" ||
          typeof target.webSocketDebuggerUrl !== "string" ||
          typeof target.url !== "string" ||
          typeof target.title !== "string" ||
          !target.title.includes("Fruitboard")
        ) {
          continue;
        }
        try {
          const url = new URL(target.url);
          if (
            (url.protocol === "http:" && url.hostname === "tauri.localhost") ||
            (url.protocol === "https:" && url.hostname === "tauri.localhost") ||
            (url.protocol === "tauri:" && url.hostname === "localhost")
          ) {
            return target;
          }
        } catch {
          // Keep looking until WebView2 exposes the application target.
        }
      }
    } catch {
      // The packaged shell may need several seconds to create WebView2.
    }
    await sleep(150);
  }
  throw new Error("the installed WebView2 target was not found");
}

function connect(target) {
  return new Promise((resolve, reject) => {
    const websocket = new WebSocket(target.webSocketDebuggerUrl);
    let nextId = 0;
    const pending = new Map();
    const connectTimer = setTimeout(
      () => reject(new Error("CDP connection timed out")),
      10000,
    );

    websocket.addEventListener("open", () => {
      clearTimeout(connectTimer);
      resolve({ websocket, call });
    });
    websocket.addEventListener("error", () => {
      clearTimeout(connectTimer);
      reject(new Error("CDP connection failed"));
    });
    websocket.addEventListener("message", (event) => {
      const message = JSON.parse(String(event.data));
      const waiter = pending.get(message.id);
      if (waiter) {
        pending.delete(message.id);
        clearTimeout(waiter.timer);
        waiter.resolve(message);
      }
    });

    function call(expression, timeoutMilliseconds = 30000) {
      return new Promise((resolveCall, rejectCall) => {
        const id = ++nextId;
        const timer = setTimeout(() => {
          pending.delete(id);
          rejectCall(new Error("CDP evaluation timed out"));
        }, timeoutMilliseconds);
        pending.set(id, { resolve: resolveCall, reject: rejectCall, timer });
        websocket.send(
          JSON.stringify({
            id,
            method: "Runtime.evaluate",
            params: {
              expression,
              returnByValue: true,
              awaitPromise: true,
            },
          }),
        );
      });
    }
  });
}

async function invoke(call, command, request) {
  const expression = `(async () => {
    try {
      const response = await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify({ request })});
      return { ok: true, response };
    } catch (error) {
      return { ok: false, error: String(error && error.message ? error.message : error) };
    }
  })()`;
  const message = await call(expression);
  if (message.result?.exceptionDetails) {
    throw new Error(`invoke ${command} evaluation failed`);
  }
  const value = message.result?.result?.value;
  if (!value) throw new Error(`invoke ${command} returned no value`);
  return value;
}

async function evaluate(call, expression, timeoutMilliseconds = 30000) {
  const message = await call(expression, timeoutMilliseconds);
  if (message.result?.exceptionDetails) {
    throw new Error(`DOM evaluation failed for installed UI observation`);
  }
  return message.result?.result?.value;
}

function compactStatus(status) {
  if (!status) return null;
  return {
    rootId: status.root?.id,
    displayName: status.root?.displayName,
    enabled: status.root?.enabled,
    state: status.state,
    jobId: status.jobId,
    runId: status.runId,
    cancellationRequested: status.cancellationRequested,
    retryAvailable: status.retryAvailable,
    counters: status.counters,
    lastSuccessfulScanAt: status.lastSuccessfulScanAt,
    errorCode: status.errorCode,
  };
}

async function listStatuses(call) {
  const result = await invoke(call, "list_scan_statuses", { schemaVersion: 1 });
  if (!result.ok || result.response.status !== "ok") {
    throw new Error(`list_scan_statuses failed`);
  }
  return result.response.data;
}

async function listRoots(call) {
  const result = await invoke(call, "list_scan_roots", { schemaVersion: 1 });
  if (!result.ok || result.response.status !== "ok") {
    throw new Error(`list_scan_roots failed`);
  }
  return result.response.data;
}

async function statusFor(call, rootId) {
  return (
    (await listStatuses(call)).find((status) => status.root.id === rootId) ??
    null
  );
}

async function waitForStatus(
  call,
  rootId,
  predicate,
  timeoutMilliseconds,
  label,
) {
  const started = Date.now();
  let lastKey = null;
  let lastStatus = null;
  while (Date.now() - started < timeoutMilliseconds) {
    lastStatus = await statusFor(call, rootId);
    const compact = compactStatus(lastStatus);
    const key = JSON.stringify(compact);
    if (key !== lastKey) {
      log({
        kind: "status-transition",
        label,
        method: "native",
        status: compact,
      });
      lastKey = key;
    }
    if (predicate(lastStatus)) return lastStatus;
    await sleep(150);
  }
  return lastStatus;
}

async function pageFor(call, rootId, limit = 20) {
  const result = await invoke(call, "get_library_page", {
    schemaVersion: 1,
    rootId,
    limit,
    cursor: null,
    snapshotId: null,
  });
  if (!result.ok || result.response.status !== "ok") {
    throw new Error(`get_library_page failed`);
  }
  return result.response.data;
}

function pageSummary(page) {
  return {
    rootId: page?.rootId,
    recordCount: page?.records?.length,
    snapshotId: page?.snapshotId,
    nextCursorPresent: page?.nextCursor !== null,
    records: page?.records?.map((record) => ({
      locationId: record.locationId,
      fileName: record.fileName,
      relativePath: record.relativePath,
      presence: record.presence,
    })),
  };
}

async function captureCommittedBaseline(call, rootId, label) {
  const status = await statusFor(call, rootId);
  const page = await pageFor(call, rootId);
  const baseline = {
    lastSuccessfulScanAt: status?.lastSuccessfulScanAt,
    page: pageSummary(page),
  };
  log({
    kind: "committed-baseline",
    method: "database",
    label,
    status: compactStatus(status),
    page: baseline.page,
  });
  return baseline;
}

async function uiSnapshot(call, label) {
  await evaluate(
    call,
    `(() => { location.hash = "#/library"; return location.hash; })()`,
  );
  await sleep(350);
  const snapshot = await evaluate(
    call,
    `(() => ({
      adapter: document.querySelector("[data-review-adapter]")?.getAttribute("data-review-adapter"),
      libraryState: document.querySelector("[data-library-state]")?.getAttribute("data-library-state"),
      progress: [...document.querySelectorAll("[data-scan-progress]")].map((element) => element.getAttribute("data-scan-progress")),
      roots: [...document.querySelectorAll("[data-root-id]")].map((element) => ({
        rootId: element.getAttribute("data-root-id"),
        progress: element.querySelector("[data-scan-progress]")?.getAttribute("data-scan-progress") ?? null,
        text: element.textContent?.trim().slice(0, 240),
      })),
      body: document.body.textContent?.slice(0, 1800),
    }))()`,
  );
  log({ kind: "ui-snapshot", label, method: "UI", snapshot });
  return snapshot;
}

function nativeCall(
  label,
  command,
  commandArguments,
  { allowFailure = false } = {},
) {
  let output = "";
  let errorOutput = "";
  let exitCode = 0;
  try {
    output = execFileSync(command, commandArguments, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
  } catch (error) {
    exitCode = Number.isInteger(error.status) ? error.status : 1;
    output = String(error.stdout ?? "");
    errorOutput = String(error.stderr ?? error.message ?? "");
  }
  const relativeArguments = commandArguments.map((value) => {
    const candidate = String(value);
    if (candidate.toLowerCase().startsWith(journeyRoot.toLowerCase())) {
      return relativeArtifact(candidate);
    }
    return candidate;
  });
  log({
    kind: "native-call",
    method: "native",
    label,
    command,
    arguments: relativeArguments,
    exitCode,
    output: `${output}${errorOutput}`.trim().slice(0, 5000),
  });
  if (exitCode !== 0 && !allowFailure) {
    throw new Error(`${label} failed with exit code ${exitCode}`);
  }
  return { exitCode, output, errorOutput };
}

function writeSyntheticFile(
  filePath,
  { exclusive = true, record = true } = {},
) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, marker, exclusive ? { flag: "wx" } : undefined);
  if (record) {
    log({
      kind: "synthetic-file-created",
      method: "native",
      path: relativeArtifact(filePath),
      bytes: fs.statSync(filePath).size,
      sha256: hashFile(filePath),
    });
  }
}

function writeManifest(directory, seed, entries) {
  const orderedEntries = [...entries].sort((left, right) =>
    left.path < right.path ? -1 : left.path > right.path ? 1 : 0,
  );
  const canonical = JSON.stringify({
    schema: "fruitboard/installed-ntfs-manifest/1",
    seed,
    entries: orderedEntries,
  });
  const manifest = {
    schema: "fruitboard/installed-ntfs-manifest/1",
    seed,
    markerSha256: createHash("sha256").update(marker).digest("hex"),
    entries: orderedEntries,
    canonicalSha256: createHash("sha256").update(canonical).digest("hex"),
  };
  const manifestPath = path.join(directory, "manifest.json");
  fs.writeFileSync(
    manifestPath,
    `${JSON.stringify(manifest, null, 2)}\n`,
    "utf8",
  );
  log({
    kind: "synthetic-manifest",
    method: "native",
    path: relativeArtifact(manifestPath),
    seed,
    entries: orderedEntries.length,
    canonicalSha256: manifest.canonicalSha256,
    fileSha256: hashFile(manifestPath),
  });
  return manifestPath;
}

function collectFlpFiles(directory) {
  const files = [];
  const visit = (current) => {
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) visit(full);
      else if (entry.isFile() && entry.name.toLowerCase().endsWith(".flp")) {
        files.push(full);
      }
    }
  };
  visit(directory);
  return files.sort((left, right) => left.localeCompare(right));
}

function prepareFixtures() {
  if (!fs.existsSync(journeyRoot) || !fs.statSync(journeyRoot).isDirectory()) {
    throw new Error(
      "the orchestration wrapper must create an existing journey root",
    );
  }
  if (!fs.existsSync(repositoryRoot)) {
    throw new Error(
      "the orchestration wrapper did not provide a repository root",
    );
  }
  const volumeRoot = path.parse(journeyRoot).root;
  const volume = nativeCall("filesystem-volume", "powershell.exe", [
    "-NoProfile",
    "-NonInteractive",
    "-Command",
    `(Get-Volume -DriveLetter ${volumeRoot[0]} | Select-Object DriveLetter,FileSystem,FileSystemLabel,HealthStatus | ConvertTo-Json -Compress)`,
  ]);
  if (!/\bFileSystem[\"']?\s*:\s*[\"']NTFS\b/i.test(volume.output)) {
    log({
      kind: "scenario-condition",
      method: "native",
      scenario: "local-ntfs",
      status: "not-qualified",
      reason: "the disposable journey root is not on an NTFS volume",
      volumeRoot,
    });
    throw new Error("the disposable journey root is not on an NTFS volume");
  }
  const generatorPath = path.join(
    repositoryRoot,
    "scripts",
    "generate-synthetic-tree.mjs",
  );
  if (!fs.existsSync(generatorPath)) {
    throw new Error("the synthetic-tree generator is missing");
  }
  const fixturesRoot = path.join(journeyRoot, "fixtures");
  if (fs.existsSync(fixturesRoot)) {
    throw new Error(
      "the fixture directory already exists; prior evidence would be overwritten",
    );
  }
  fs.mkdirSync(fixturesRoot, { recursive: true });

  const generatedRoot = path.join(fixturesRoot, "resource-baseline");
  const generatorOutput = execFileSync(
    process.execPath,
    [
      generatorPath,
      "--size",
      "baseline",
      "--seed",
      "20260912",
      "--out",
      generatedRoot,
    ],
    { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], windowsHide: true },
  );
  const manifestPath = path.join(generatedRoot, "manifest.json");
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const resourceRoot = path.join(generatedRoot, "roots");
  const resourceFiles = collectFlpFiles(resourceRoot);
  if (
    manifest.counts?.flpFiles !== 10000 ||
    manifest.counts?.aliasLocations !== 5 ||
    resourceFiles.length !== 8000
  ) {
    throw new Error(
      "the accepted baseline fixture shape changed; the quota-fitting resource subroot is not reproducible",
    );
  }
  const generatorHash = /manifest sha-256:\s*([0-9a-f]+)/i.exec(
    generatorOutput,
  )?.[1];
  if (!generatorHash)
    throw new Error("the generator did not report a manifest hash");
  log({
    kind: "synthetic-generator",
    method: "native",
    command: "generate-synthetic-tree.mjs",
    arguments: [
      "--size",
      "baseline",
      "--seed",
      "20260912",
      "--out",
      "fixtures/resource-baseline",
    ],
    output: generatorOutput.trim(),
    generatorManifestSha256: generatorHash,
    manifestFileSha256: hashFile(manifestPath),
    counts: manifest.counts,
    scannedRoot: "fixtures/resource-baseline/roots",
    scannedFlpCount: resourceFiles.length,
    note: "The accepted 10,000-file manifest is retained; its roots subroot contains 8,000 primary locations so the safety baseline can commit under the existing 10,000-record bound without changing the accepted fixture or budget.",
  });

  const resourceOverflowDirectory = path.join(
    resourceRoot,
    "resource-overflow-20260912",
  );
  const resourceOverflowCount = 10000 - resourceFiles.length + 1;
  if (resourceOverflowCount < 1 || fs.existsSync(resourceOverflowDirectory)) {
    throw new Error(
      "the resource overflow fixture is not disposable and reproducible",
    );
  }

  const deniedDirectory = path.join(fixturesRoot, "denied", "scan-root");
  const deniedEntries = [
    [
      "allowed/keep-one.flp",
      path.join(deniedDirectory, "allowed", "keep-one.flp"),
    ],
    [
      "allowed/keep-two.flp",
      path.join(deniedDirectory, "allowed", "keep-two.flp"),
    ],
    [
      "blocked/secret-one.flp",
      path.join(deniedDirectory, "blocked", "secret-one.flp"),
    ],
    [
      "blocked/secret-two.flp",
      path.join(deniedDirectory, "blocked", "secret-two.flp"),
    ],
  ];
  for (const [, filePath] of deniedEntries) writeSyntheticFile(filePath);
  const deniedManifest = writeManifest(
    path.join(fixturesRoot, "denied"),
    "denied-20260912",
    deniedEntries.map(([relativePath, filePath]) => ({
      path: relativePath,
      bytes: fs.statSync(filePath).size,
      kind: "flp",
    })),
  );

  const hardlinkBase = path.join(fixturesRoot, "hardlinks");
  const hardlinkRoot = path.join(hardlinkBase, "scan-root");
  const hardlinkSource = path.join(hardlinkBase, "backing", "shared.flp");
  const hardlinkA = path.join(hardlinkRoot, "alias-a", "shared.flp");
  const hardlinkB = path.join(hardlinkRoot, "alias-b", "shared.flp");
  writeSyntheticFile(hardlinkSource);
  fs.mkdirSync(path.dirname(hardlinkA), { recursive: true });
  fs.mkdirSync(path.dirname(hardlinkB), { recursive: true });
  fs.linkSync(hardlinkSource, hardlinkA);
  fs.linkSync(hardlinkSource, hardlinkB);
  const sourceStat = fs.statSync(hardlinkSource);
  const aliasAStat = fs.statSync(hardlinkA);
  const aliasBStat = fs.statSync(hardlinkB);
  const hardlinkList = nativeCall(
    "hardlink-list-alias-a",
    "fsutil.exe",
    ["hardlink", "list", hardlinkA],
    { allowFailure: true },
  );
  if (hardlinkList.exitCode !== 0) {
    log({
      kind: "scenario-condition",
      method: "native",
      scenario: "hardlinks",
      status: "not-qualified",
      reason: "fsutil hardlink list was unavailable or failed",
    });
  }
  log({
    kind: "hardlink-fixture",
    method: "native",
    root: "fixtures/hardlinks/scan-root",
    source: "fixtures/hardlinks/backing/shared.flp",
    aliases: [
      "fixtures/hardlinks/scan-root/alias-a/shared.flp",
      "fixtures/hardlinks/scan-root/alias-b/shared.flp",
    ],
    sourceStat: {
      dev: String(sourceStat.dev),
      ino: String(sourceStat.ino),
      bytes: sourceStat.size,
    },
    aliasAStat: {
      dev: String(aliasAStat.dev),
      ino: String(aliasAStat.ino),
      bytes: aliasAStat.size,
    },
    aliasBStat: {
      dev: String(aliasBStat.dev),
      ino: String(aliasBStat.ino),
      bytes: aliasBStat.size,
    },
    distinctAliasPaths: hardlinkA !== hardlinkB,
    manifest: relativeArtifact(
      writeManifest(
        path.join(fixturesRoot, "hardlinks"),
        "hardlinks-20260912",
        [
          {
            path: "scan-root/alias-a/shared.flp",
            bytes: aliasAStat.size,
            kind: "flp",
            hardlinkGroup: "shared-1",
            hardlinkRole: "primary",
          },
          {
            path: "scan-root/alias-b/shared.flp",
            bytes: aliasBStat.size,
            kind: "flp",
            hardlinkGroup: "shared-1",
            hardlinkRole: "alias",
          },
        ],
      ),
    ),
  });

  return {
    denied: {
      root: deniedDirectory,
      blocked: path.join(deniedDirectory, "blocked"),
      manifest: deniedManifest,
    },
    resource: {
      root: resourceRoot,
      manifest: manifestPath,
      generatorManifestSha256: generatorHash,
      baselineCount: resourceFiles.length,
      overflowDirectory: resourceOverflowDirectory,
      overflowCount: resourceOverflowCount,
    },
    hardlinks: {
      root: hardlinkRoot,
      source: hardlinkSource,
      aliasA: hardlinkA,
      aliasB: hardlinkB,
    },
  };
}

function writeResourceOverflowFixture(resource) {
  fs.mkdirSync(resource.overflowDirectory, { recursive: true });
  const entries = [];
  for (let index = 0; index < resource.overflowCount; index += 1) {
    const name = `overflow-${String(index).padStart(4, "0")}.flp`;
    const filePath = path.join(resource.overflowDirectory, name);
    writeSyntheticFile(filePath, { record: false });
    entries.push({
      path: path.relative(resource.root, filePath).split(path.sep).join("/"),
      bytes: fs.statSync(filePath).size,
      kind: "flp",
      sha256: hashFile(filePath),
    });
  }
  const evidenceDirectory = path.join(
    path.dirname(resource.manifest),
    "resource-overflow-evidence",
  );
  fs.mkdirSync(evidenceDirectory, { recursive: true });
  const manifestPath = writeManifest(
    evidenceDirectory,
    "resource-overflow-20260912",
    entries,
  );
  log({
    kind: "resource-overflow-fixture",
    method: "native",
    root: relativeArtifact(resource.root),
    directory: relativeArtifact(resource.overflowDirectory),
    baselineCount: resource.baselineCount,
    overflowCount: resource.overflowCount,
    expectedObservedCount: resource.baselineCount + resource.overflowCount,
    manifest: relativeArtifact(manifestPath),
    manifestSha256: hashFile(manifestPath),
  });
  return manifestPath;
}

function compactRunResult(start, terminal, page) {
  return {
    requestedJobId: start.response?.data?.jobId,
    completedJobId: terminal?.jobId,
    successorObserved:
      Boolean(start.response?.data?.jobId) &&
      start.response?.data?.jobId !== terminal?.jobId,
    status: compactStatus(terminal),
    page: pageSummary(page),
  };
}

async function scanToTerminal(
  call,
  rootId,
  label,
  timeoutMilliseconds = 240000,
) {
  const start = await invoke(call, "scan_now", { schemaVersion: 1, rootId });
  log({ kind: `${label}-scan-now`, method: "native", result: start });
  if (!start.ok || start.response.status !== "ok") {
    throw new Error(
      `${label} scan_now did not return a successful typed response`,
    );
  }
  let terminal = await waitForStatus(
    call,
    rootId,
    (status) =>
      ["completed", "failed", "cancelled", "interrupted"].includes(
        status?.state,
      ) && status.retryAvailable === false,
    timeoutMilliseconds,
    label,
  );
  // The worker commits the durable terminal state before the host records its
  // in-memory completion counters. Re-read after the transition so a valid
  // completed publication is not mistaken for a zero-counter race.
  await sleep(250);
  const refreshed = await statusFor(call, rootId);
  if (
    refreshed &&
    ["completed", "failed", "cancelled", "interrupted"].includes(
      refreshed.state,
    ) &&
    refreshed.retryAvailable === false
  ) {
    terminal = refreshed;
  }
  if (
    !terminal ||
    !["completed", "failed", "cancelled", "interrupted"].includes(
      terminal.state,
    )
  ) {
    throw new Error(
      `${label} did not reach a settled terminal status for its requested scan chain`,
    );
  }
  const page = await pageFor(call, rootId);
  log({
    kind: `${label}-terminal`,
    method: "native",
    ...compactRunResult(start, terminal, page),
  });
  return { start, status: terminal, page };
}

async function toggleRoot(call, rootId, enabled, label) {
  const result = await invoke(call, "set_scan_root_enabled", {
    schemaVersion: 1,
    id: rootId,
    enabled,
  });
  log({
    kind: "root-toggle",
    method: "native",
    label,
    rootId,
    enabled,
    result,
  });
  if (
    !result.ok ||
    result.response.status !== "ok" ||
    result.response.data.enabled !== enabled
  ) {
    throw new Error(`${label} did not persist the requested root state`);
  }
  await sleep(500);
  return result.response.data;
}

async function addRoot(call, displayName, rootPath) {
  const result = await invoke(call, "add_scan_root", {
    schemaVersion: 1,
    displayName,
    path: rootPath,
  });
  log({
    kind: "root-added",
    method: "native",
    displayName,
    relativePath: relativeArtifact(rootPath),
    result,
  });
  if (!result.ok || result.response.status !== "ok") {
    throw new Error(`failed to add ${displayName}`);
  }
  return result.response.data;
}

async function identifyCurrentUser() {
  const result = nativeCall("acl-identity", "whoami.exe", []);
  const identity = result.output.trim().split(/\r?\n/u)[0];
  if (!identity) throw new Error("whoami returned no ACL identity");
  return identity;
}

function assertDirectoryDenied(directory) {
  try {
    fs.readdirSync(directory);
    log({
      kind: "denial-probe",
      method: "native",
      path: relativeArtifact(directory),
      denied: false,
    });
    throw new Error(
      "the same-user native denial probe could still enumerate the protected directory",
    );
  } catch (error) {
    if (error.message.includes("same-user native denial probe")) throw error;
    const denied = ["EACCES", "EPERM"].includes(error.code);
    log({
      kind: "denial-probe",
      method: "native",
      path: relativeArtifact(directory),
      denied,
      errorCode: error.code ?? null,
      error: safeError(error),
    });
    if (!denied)
      throw new Error(
        "the native denial probe failed for a reason other than EACCES/EPERM",
      );
  }
}

function restoreAcl(directory, identity) {
  if (!fs.existsSync(directory)) return;
  const result = nativeCall(
    "acl-restore",
    "icacls.exe",
    [directory, "/remove:d", identity, "/T", "/C"],
    { allowFailure: true },
  );
  let restored = false;
  try {
    fs.readdirSync(directory);
    restored = true;
  } catch {
    restored = false;
  }
  log({
    kind: "acl-restore-result",
    method: "native",
    path: relativeArtifact(directory),
    commandExitCode: result.exitCode,
    restored,
  });
  if (result.exitCode !== 0 || !restored) {
    throw new Error(
      "disposable ACL restoration did not restore same-user traversal",
    );
  }
}

function copyDatabaseSnapshot(snapshotName) {
  assertLockOwnership();
  const sourceDirectory = storageDirectory;
  if (!fs.existsSync(sourceDirectory)) {
    throw new Error(
      "the dedicated Foundation Smoke storage directory is missing",
    );
  }
  const destinationDirectory = path.join(
    journeyRoot,
    `database-${snapshotName}`,
  );
  if (fs.existsSync(destinationDirectory)) {
    throw new Error(`database snapshot already exists: ${snapshotName}`);
  }
  fs.mkdirSync(destinationDirectory, { recursive: true });
  const copied = [];
  for (const name of [
    "fruitboard.db",
    "fruitboard.db-wal",
    "fruitboard.db-shm",
  ]) {
    const source = path.join(sourceDirectory, name);
    if (!fs.existsSync(source)) continue;
    const destination = path.join(destinationDirectory, name);
    fs.copyFileSync(source, destination);
    copied.push({
      name,
      bytes: fs.statSync(destination).size,
      sha256: hashFile(destination),
    });
  }
  if (!copied.some((file) => file.name === "fruitboard.db")) {
    throw new Error(`database snapshot ${snapshotName} has no fruitboard.db`);
  }
  log({
    kind: "database-snapshot",
    method: "database",
    snapshot: snapshotName,
    directory: relativeArtifact(destinationDirectory),
    files: copied,
  });
  return destinationDirectory;
}

async function startApp(label) {
  assertLockOwnership();
  if (!fs.existsSync(appPath))
    throw new Error("the installed executable is missing");
  const port = await getFreePort();
  const child = spawn(appPath, [], {
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    },
    stdio: "ignore",
    windowsHide: true,
  });
  log({ kind: "app-spawned", method: "native", label, pid: child.pid, port });
  try {
    const target = await discover(port);
    const connection = await connect(target);
    log({
      kind: "app-ready",
      method: "UI",
      label,
      title: target.title,
      urlProtocol: new URL(target.url).protocol,
    });
    const consoleState = await invoke(
      connection.call,
      "get_scan_console_state",
      { schemaVersion: 1 },
    );
    if (
      !consoleState.ok ||
      consoleState.response.status !== "ok" ||
      !consoleState.response.data.enabled
    ) {
      throw new Error("the installed package did not enable scan-console");
    }
    log({
      kind: "scan-console-state",
      method: "native",
      label,
      result: consoleState,
    });
    return { child, websocket: connection.websocket, call: connection.call };
  } catch (error) {
    try {
      execFileSync("taskkill.exe", ["/F", "/PID", String(child.pid)], {
        stdio: "ignore",
        windowsHide: true,
      });
    } catch {
      // Preserve the original discovery error.
    }
    throw error;
  }
}

async function stopApp(app, label) {
  if (!app?.child?.pid) return;
  try {
    app.websocket?.close();
  } catch {
    // The CDP target can disappear during normal close.
  }
  const pid = app.child.pid;
  const graceful = nativeCall(
    "app-stop",
    "taskkill.exe",
    ["/PID", String(pid)],
    { allowFailure: true },
  );
  const started = Date.now();
  while (isProcessAlive(pid) && Date.now() - started < 10000) await sleep(100);
  let forced = false;
  if (isProcessAlive(pid)) {
    forced = true;
    nativeCall("app-stop-force", "taskkill.exe", ["/F", "/PID", String(pid)]);
    const forceStarted = Date.now();
    while (isProcessAlive(pid) && Date.now() - forceStarted < 5000)
      await sleep(100);
  }
  if (isProcessAlive(pid))
    throw new Error(`${label} app process survived exact-PID cleanup`);
  log({
    kind: "app-stopped",
    method: "native",
    label,
    pid,
    gracefulExitCode: graceful.exitCode,
    forced,
  });
}

async function runCase(label, cases, body) {
  try {
    await body();
    log({ kind: "case-result", method: "native", scenario: label, pass: true });
  } catch (error) {
    cases.push({ scenario: label, error: safeError(error) });
    log({
      kind: "case-result",
      method: "native",
      scenario: label,
      pass: false,
      error: safeError(error),
    });
  }
}

async function main() {
  assertLockOwnership();
  if (
    path.basename(dataDirectory).toLowerCase() !==
    "com.fruitboard.desktop.foundation-smoke"
  ) {
    throw new Error(
      "the driver received a non-dedicated Foundation Smoke data directory",
    );
  }
  if (fs.existsSync(transcriptPath)) {
    throw new Error(
      "the transcript already exists; refusing to overwrite prior evidence",
    );
  }
  if (!fs.existsSync(appPath) || !fs.statSync(appPath).isFile()) {
    throw new Error("the installed app path is not a file");
  }
  log({
    kind: "driver-start",
    driver: "installed-ntfs-cases.mjs",
    schemaVersion: 1,
    app: path.basename(appPath),
    dataDirectory: "com.fruitboard.desktop.foundation-smoke",
    lockOwnerPid: expectedLockPid,
    historicalDriver: "queued-state-restart.mjs (preserved; not invoked)",
  });

  const fixtures = prepareFixtures();
  const failures = [];
  let app;
  let roots;
  let aclIdentity;
  try {
    app = await startApp("initial");
    const initialRoots = await listRoots(app.call);
    log({
      kind: "initial-roots",
      method: "native",
      count: initialRoots.length,
    });
    if (initialRoots.length !== 0)
      throw new Error("the installed dedicated database was not empty");
    roots = {
      denied: await addRoot(app.call, "Denied Traversal", fixtures.denied.root),
      resource: await addRoot(
        app.call,
        "Resource Limit",
        fixtures.resource.root,
      ),
      hardlinks: await addRoot(
        app.call,
        "Hardlink Aliases",
        fixtures.hardlinks.root,
      ),
    };

    const baselines = {};
    await runCase("baseline-commits", failures, async () => {
      const denied = await scanToTerminal(
        app.call,
        roots.denied.id,
        "denied-baseline",
      );
      if (denied.status.state !== "completed")
        throw new Error("denied fixture baseline did not complete");
      baselines.denied = {
        lastSuccessfulScanAt: denied.status.lastSuccessfulScanAt,
        page: pageSummary(denied.page),
      };
      const hardlinks = await scanToTerminal(
        app.call,
        roots.hardlinks.id,
        "hardlinks-baseline",
      );
      if (
        hardlinks.status.state !== "completed" ||
        hardlinks.page.records.length !== 2
      ) {
        throw new Error(
          "hardlink baseline did not publish exactly two locations",
        );
      }
      const hardlinkPaths = new Set(
        hardlinks.page.records.map((record) => record.relativePath),
      );
      if (
        !hardlinkPaths.has("alias-a\\shared.flp") ||
        !hardlinkPaths.has("alias-b\\shared.flp")
      ) {
        throw new Error(
          "hardlink baseline did not publish both scanned aliases",
        );
      }
      baselines.hardlinks = {
        lastSuccessfulScanAt: hardlinks.status.lastSuccessfulScanAt,
        page: hardlinks.page,
      };
      const resource = await scanToTerminal(
        app.call,
        roots.resource.id,
        "resource-baseline",
        300000,
      );
      if (
        resource.status.state !== "completed" ||
        resource.status.counters?.filesObserved !==
          fixtures.resource.baselineCount
      ) {
        throw new Error(
          `the quota-fitting resource baseline did not complete with ${fixtures.resource.baselineCount} observations`,
        );
      }
      baselines.resource = {
        lastSuccessfulScanAt: resource.status.lastSuccessfulScanAt,
        page: pageSummary(resource.page),
      };
      await uiSnapshot(app.call, "after-baseline-commits");
    });

    await stopApp(app, "after-baseline");
    app = undefined;
    copyDatabaseSnapshot("baseline");
    app = await startApp("after-baseline");

    await runCase("denied-traversal", failures, async () => {
      if (!baselines.denied) {
        throw new Error(
          "not exercised: denied traversal has no committed baseline",
        );
      }
      aclIdentity = await identifyCurrentUser();
      await toggleRoot(
        app.call,
        roots.denied.id,
        false,
        "denied-disable-before-acl",
      );
      baselines.denied = await captureCommittedBaseline(
        app.call,
        roots.denied.id,
        "before-denied-acl",
      );
      nativeCall("acl-deny", "icacls.exe", [
        fixtures.denied.blocked,
        "/deny",
        `${aclIdentity}:(OI)(CI)(RX)`,
      ]);
      assertDirectoryDenied(fixtures.denied.blocked);
      try {
        await toggleRoot(
          app.call,
          roots.denied.id,
          true,
          "denied-enable-after-acl",
        );
        const denied = await scanToTerminal(
          app.call,
          roots.denied.id,
          "denied-after-acl",
        );
        if (
          denied.status.state !== "failed" ||
          denied.status.errorCode !== "access_denied"
        ) {
          throw new Error(
            `the installed status was not failed/access_denied: ${JSON.stringify(compactStatus(denied.status))}`,
          );
        }
        if (
          denied.status.lastSuccessfulScanAt !==
          baselines.denied.lastSuccessfulScanAt
        ) {
          throw new Error("denied traversal changed the last-success marker");
        }
        if (
          denied.page.records.some((record) => record.presence !== "present") ||
          denied.page.records.length !== baselines.denied.page.recordCount
        ) {
          throw new Error(
            "denied traversal changed committed presence or row count",
          );
        }
        await uiSnapshot(app.call, "after-denied-traversal");
      } finally {
        await toggleRoot(
          app.call,
          roots.denied.id,
          false,
          "denied-disable-before-acl-restore",
        );
        restoreAcl(fixtures.denied.blocked, aclIdentity);
      }
    });

    await stopApp(app, "after-denied");
    app = undefined;
    copyDatabaseSnapshot("after-denied");
    app = await startApp("after-denied");

    await runCase("resource-limit", failures, async () => {
      if (!baselines.resource) {
        throw new Error(
          "not exercised: ResourceLimit has no committed baseline",
        );
      }
      await toggleRoot(
        app.call,
        roots.resource.id,
        false,
        "resource-disable-before-overflow",
      );
      baselines.resource = await captureCommittedBaseline(
        app.call,
        roots.resource.id,
        "before-resource-overflow",
      );
      writeResourceOverflowFixture(fixtures.resource);
      try {
        await toggleRoot(
          app.call,
          roots.resource.id,
          true,
          "resource-enable-after-overflow",
        );
        const limited = await scanToTerminal(
          app.call,
          roots.resource.id,
          "resource-over-bound",
          300000,
        );
        if (
          limited.status.state !== "failed" ||
          limited.status.errorCode !== "resource_limit"
        ) {
          throw new Error(
            `the installed status was not failed/resource_limit: ${JSON.stringify(compactStatus(limited.status))}`,
          );
        }
        if (
          limited.status.lastSuccessfulScanAt !==
          baselines.resource.lastSuccessfulScanAt
        ) {
          throw new Error("resource limit changed the last-success marker");
        }
        if (
          limited.page.records.some((record) => record.presence !== "present")
        ) {
          throw new Error(
            "resource limit caused a false missing transition in the visible page",
          );
        }
        await uiSnapshot(app.call, "after-resource-limit");
      } finally {
        await toggleRoot(
          app.call,
          roots.resource.id,
          false,
          "resource-disable-after-limit",
        );
      }
    });

    await stopApp(app, "after-resource-limit");
    app = undefined;
    copyDatabaseSnapshot("after-resource-limit");
    app = await startApp("after-resource-limit");

    await runCase("hardlink-aliases", failures, async () => {
      if (!baselines.hardlinks) {
        throw new Error(
          "not exercised: hardlink aliases have no committed baseline",
        );
      }
      await toggleRoot(
        app.call,
        roots.hardlinks.id,
        false,
        "hardlinks-disable-before-remove",
      );
      baselines.hardlinks = await captureCommittedBaseline(
        app.call,
        roots.hardlinks.id,
        "before-hardlink-remove",
      );
      fs.unlinkSync(fixtures.hardlinks.aliasA);
      log({
        kind: "hardlink-alias-removed",
        method: "native",
        relativePath: relativeArtifact(fixtures.hardlinks.aliasA),
      });
      nativeCall(
        "hardlink-list-surviving-alias",
        "fsutil.exe",
        ["hardlink", "list", fixtures.hardlinks.aliasB],
        { allowFailure: true },
      );
      await toggleRoot(
        app.call,
        roots.hardlinks.id,
        true,
        "hardlinks-enable-after-remove",
      );
      const missing = await scanToTerminal(
        app.call,
        roots.hardlinks.id,
        "hardlinks-after-remove",
      );
      if (missing.status.state !== "completed")
        throw new Error("hardlink removal scan did not complete");
      const missingByPath = new Map(
        missing.page.records.map((record) => [
          record.relativePath,
          record.presence,
        ]),
      );
      if (
        missingByPath.get("alias-a\\shared.flp") !== "missing" ||
        missingByPath.get("alias-b\\shared.flp") !== "present"
      ) {
        throw new Error(
          "removing one hardlink alias did not preserve the other location",
        );
      }
      await uiSnapshot(app.call, "after-hardlink-remove");

      await toggleRoot(
        app.call,
        roots.hardlinks.id,
        false,
        "hardlinks-disable-before-restore",
      );
      fs.linkSync(fixtures.hardlinks.source, fixtures.hardlinks.aliasA);
      log({
        kind: "hardlink-alias-restored",
        method: "native",
        relativePath: relativeArtifact(fixtures.hardlinks.aliasA),
      });
      nativeCall(
        "hardlink-list-restored-alias",
        "fsutil.exe",
        ["hardlink", "list", fixtures.hardlinks.aliasA],
        { allowFailure: true },
      );
      await toggleRoot(
        app.call,
        roots.hardlinks.id,
        true,
        "hardlinks-enable-after-restore",
      );
      const restored = await scanToTerminal(
        app.call,
        roots.hardlinks.id,
        "hardlinks-after-restore",
      );
      if (restored.status.state !== "completed")
        throw new Error("hardlink restoration scan did not complete");
      const restoredByPath = new Map(
        restored.page.records.map((record) => [record.relativePath, record]),
      );
      if (
        restoredByPath.size !== 2 ||
        [...restoredByPath.values()].some(
          (record) => record.presence !== "present",
        )
      ) {
        throw new Error(
          "restoring one hardlink alias did not restore both independent locations",
        );
      }
      if (
        new Set([...restoredByPath.values()].map((record) => record.locationId))
          .size !== 2
      ) {
        throw new Error("hardlink aliases collapsed to one visible location");
      }
      await uiSnapshot(app.call, "after-hardlink-restore");
      await toggleRoot(
        app.call,
        roots.hardlinks.id,
        false,
        "hardlinks-disable-after-restore",
      );
    });
  } finally {
    if (app) {
      try {
        await stopApp(app, "final-cleanup");
      } catch (error) {
        failures.push({ scenario: "final-cleanup", error: safeError(error) });
      }
    }
    if (aclIdentity) {
      try {
        restoreAcl(fixtures.denied.blocked, aclIdentity);
      } catch (error) {
        failures.push({ scenario: "acl-cleanup", error: safeError(error) });
      }
    }
    if (fs.existsSync(fixtures.hardlinks.aliasA) === false) {
      try {
        fs.linkSync(fixtures.hardlinks.source, fixtures.hardlinks.aliasA);
        log({
          kind: "hardlink-cleanup-restored",
          method: "native",
          relativePath: relativeArtifact(fixtures.hardlinks.aliasA),
        });
      } catch (error) {
        failures.push({
          scenario: "hardlink-cleanup",
          error: safeError(error),
        });
      }
    }
  }

  const finalSnapshot = copyDatabaseSnapshot("final");
  log({
    kind: "driver-result",
    pass: failures.length === 0,
    failures,
    databaseSnapshot: relativeArtifact(finalSnapshot),
    qualifiedFilesystem: "local NTFS only",
    excludedFilesystemClaims: ["FAT32", "DriveFS", "network shares"],
  });
  if (failures.length > 0) {
    throw new Error(
      `${failures.length} installed NTFS case(s) failed; see ntfs-cases.jsonl`,
    );
  }
}

try {
  await main();
} catch (error) {
  try {
    log({ kind: "driver-result", pass: false, error: safeError(error) });
  } catch {
    // Keep the process failure visible even if the transcript could not open.
  }
  console.error(error);
  process.exitCode = 1;
}
