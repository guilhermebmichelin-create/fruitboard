import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";

// Review-only driver for the installed Foundation Smoke package. It deliberately
// keeps the target root free of scan_now calls after the target lease is seen.
const journeyRoot = path.resolve(process.argv[2] ?? "");
const appPath = path.resolve(
  process.argv[3] ??
    path.join(journeyRoot, "Installed Fruitboard", "fruitboard-desktop.exe"),
);
const transcriptPath = path.join(journeyRoot, "queued-state-restart.jsonl");
const dataDirectory = path.join(
  process.env.LOCALAPPDATA ?? "",
  "com.fruitboard.desktop.foundation-smoke",
);
const storageDirectory = path.join(dataDirectory, "storage");

if (!journeyRoot || !fs.existsSync(appPath)) {
  throw new Error(
    "usage: node queued-state-restart.mjs <journey-root> [app-path]",
  );
}

fs.mkdirSync(journeyRoot, { recursive: true });

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
          // Keep looking until the browser exposes the application target.
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

    function call(expression, timeoutMilliseconds = 20000) {
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
  const value = message.result?.result?.value;
  if (!value) {
    throw new Error(`invoke ${command} returned no value`);
  }
  return value;
}

async function evaluate(call, expression, timeoutMilliseconds = 20000) {
  const message = await call(expression, timeoutMilliseconds);
  if (message.result?.exceptionDetails) {
    throw new Error(
      `DOM evaluation failed: ${JSON.stringify(message.result.exceptionDetails)}`,
    );
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
    errorCode: status.errorCode,
  };
}

async function listStatuses(call) {
  const result = await invoke(call, "list_scan_statuses", { schemaVersion: 1 });
  if (!result.ok || result.response.status !== "ok") {
    throw new Error(`list_scan_statuses failed: ${JSON.stringify(result)}`);
  }
  return result.response.data;
}

async function listRoots(call) {
  const result = await invoke(call, "list_scan_roots", { schemaVersion: 1 });
  if (!result.ok || result.response.status !== "ok") {
    throw new Error(`list_scan_roots failed: ${JSON.stringify(result)}`);
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
    await sleep(100);
  }
  return lastStatus;
}

async function getPage(call, rootId, limit = 1) {
  return invoke(call, "get_library_page", {
    schemaVersion: 1,
    rootId,
    limit,
    cursor: null,
    snapshotId: null,
  });
}

function pageSummary(page) {
  if (!page?.ok) return page;
  const response = page.response;
  return {
    status: response?.status,
    rootId: response?.data?.rootId,
    recordCount: response?.data?.records?.length,
    snapshotId: response?.data?.snapshotId,
    nextCursorPresent: response?.data?.nextCursor !== null,
    firstNames: response?.data?.records
      ?.slice(0, 3)
      .map((record) => record.fileName),
  };
}

async function scanToCompleted(
  call,
  rootId,
  label,
  timeoutMilliseconds = 180000,
) {
  const start = await invoke(call, "scan_now", { schemaVersion: 1, rootId });
  log({ kind: `${label}-scan-now`, method: "native", result: start });
  if (!start.ok || start.response.status !== "ok") {
    throw new Error(`${label} scan_now failed`);
  }
  const jobId = start.response.data.jobId;
  const final = await waitForStatus(
    call,
    rootId,
    (status) => status?.state === "completed",
    timeoutMilliseconds,
    label,
  );
  if (!final || final.state !== "completed") {
    throw new Error(
      `${label} did not complete: ${JSON.stringify(compactStatus(final))}`,
    );
  }
  const page = await getPage(call, rootId);
  log({
    kind: `${label}-completed`,
    method: "native",
    requestedJobId: jobId,
    completedJobId: final.jobId,
    successorObserved: final.jobId !== jobId,
    status: compactStatus(final),
    page: pageSummary(page),
  });
  return { jobId, completedJobId: final.jobId, status: final };
}

function launchApp() {
  return getFreePort().then((port) => {
    const child = spawn(appPath, [], {
      env: {
        ...process.env,
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
      },
      stdio: "ignore",
    });
    log({ kind: "app-spawned", method: "native", pid: child.pid, port });
    return { child, port };
  });
}

function isProcessAlive(pid) {
  if (!pid) return false;
  try {
    const output = execFileSync(
      "tasklist",
      ["/FI", `PID eq ${pid}`, "/FO", "CSV", "/NH"],
      { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    );
    return output.includes(`,\"${pid}\",`);
  } catch {
    return false;
  }
}

async function waitForProcessExit(child, timeoutMilliseconds = 5000) {
  const started = Date.now();
  while (
    child.exitCode === null &&
    Date.now() - started < timeoutMilliseconds
  ) {
    await sleep(100);
  }
  return child.exitCode !== null;
}

async function stopExactProcess(child, force = false) {
  if (!child?.pid || !isProcessAlive(child.pid)) return;
  try {
    execFileSync(
      "taskkill",
      force ? ["/F", "/PID", String(child.pid)] : ["/PID", String(child.pid)],
      { stdio: "ignore" },
    );
  } catch {
    // The process may have exited between the tasklist and taskkill calls.
  }
  await waitForProcessExit(child, force ? 3000 : 5000);
  if (isProcessAlive(child.pid) && !force) {
    await stopExactProcess(child, true);
  }
}

function copyDurableState(destinationDirectory) {
  fs.mkdirSync(destinationDirectory, { recursive: true });
  const copied = [];
  for (const name of [
    "fruitboard.db",
    "fruitboard.db-wal",
    "fruitboard.db-shm",
  ]) {
    const source = path.join(storageDirectory, name);
    if (!fs.existsSync(source)) continue;
    const destination = path.join(destinationDirectory, name);
    fs.copyFileSync(source, destination);
    copied.push({
      name,
      bytes: fs.statSync(destination).size,
      sha256: createHash("sha256")
        .update(fs.readFileSync(destination))
        .digest("hex"),
    });
  }
  if (!copied.some((file) => file.name === "fruitboard.db")) {
    throw new Error("the database was not present immediately after hard kill");
  }
  return copied;
}

async function libraryUiSnapshot(call, label) {
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
        text: element.textContent?.trim().slice(0, 300),
      })),
      body: document.body.textContent?.slice(0, 1800),
    }))()`,
  );
  log({ kind: "ui-snapshot", label, method: "UI", snapshot });
  return snapshot;
}

async function main() {
  log({
    kind: "driver-start",
    driver: "queued-state-restart.mjs",
    schemaVersion: 1,
    appPath,
    dataDirectory,
  });

  let firstApp;
  let firstConnection;
  let secondApp;
  let secondConnection;
  let targetJobId;
  let targetRunId;

  try {
    firstApp = await launchApp();
    await sleep(3500);
    const target = await discover(firstApp.port);
    firstConnection = await connect(target);
    const { websocket, call } = firstConnection;
    log({
      kind: "app-ready",
      method: "UI",
      url: target.url,
      title: target.title,
    });
    const consoleState = await invoke(call, "get_scan_console_state", {
      schemaVersion: 1,
    });
    log({ kind: "scan-console-state", method: "native", result: consoleState });
    if (
      !consoleState.ok ||
      consoleState.response.status !== "ok" ||
      !consoleState.response.data.enabled
    ) {
      throw new Error("the installed package did not enable scan-console");
    }

    const initialRoots = await listRoots(call);
    log({
      kind: "initial-roots",
      method: "native",
      count: initialRoots.length,
    });
    if (initialRoots.length !== 0) {
      throw new Error("the installed database was not isolated and empty");
    }

    const primaryRootsDirectory = path.join(journeyRoot, "fixture", "roots");
    const leafDirectories = fs
      .readdirSync(primaryRootsDirectory, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name)
      .sort();
    if (leafDirectories.length < 2) {
      throw new Error(
        "the synthetic fixture did not provide two sibling leaves",
      );
    }
    const primaryLeaf = path.join(primaryRootsDirectory, leafDirectories[0]);
    const removeLeaf = path.join(primaryRootsDirectory, leafDirectories[1]);
    const targetPath = path.join(journeyRoot, "cancellation-fixture", "roots");

    async function addRoot(displayName, rootPath) {
      const result = await invoke(call, "add_scan_root", {
        schemaVersion: 1,
        displayName,
        path: rootPath,
      });
      log({
        kind: "root-added",
        method: "native",
        displayName,
        relativePath: path.relative(journeyRoot, rootPath),
        result,
      });
      if (!result.ok || result.response.status !== "ok") {
        throw new Error(`failed to add ${displayName}`);
      }
      return result.response.data;
    }

    const targetRoot = await addRoot("Restart Target", targetPath);
    const disableRoot = await addRoot("Queued Disable", primaryLeaf);
    const removeRoot = await addRoot("Queued Remove", removeLeaf);
    fs.writeFileSync(
      path.join(journeyRoot, "root-ids.json"),
      JSON.stringify(
        {
          targetId: targetRoot.id,
          disableId: disableRoot.id,
          removeId: removeRoot.id,
          targetRelativePath: path.relative(journeyRoot, targetPath),
          disableRelativePath: path.relative(journeyRoot, primaryLeaf),
          removeRelativePath: path.relative(journeyRoot, removeLeaf),
        },
        null,
        2,
      ),
      "utf8",
    );

    // Seed committed rows for the target and both queued-action roots. These
    // scans complete before the hard-kill scenario begins.
    await scanToCompleted(call, targetRoot.id, "baseline-target");
    await scanToCompleted(call, disableRoot.id, "baseline-disable");
    await scanToCompleted(call, removeRoot.id, "baseline-remove");
    await libraryUiSnapshot(call, "after-baselines");

    // One target scan_now only. Once this returns Running with a non-null run,
    // this driver never calls scan_now for targetRoot and never mutates its
    // fixture before the hard kill.
    const targetStart = await invoke(call, "scan_now", {
      schemaVersion: 1,
      rootId: targetRoot.id,
    });
    log({ kind: "target-scan-now", method: "native", result: targetStart });
    if (!targetStart.ok || targetStart.response.status !== "ok") {
      throw new Error("target scan_now failed");
    }
    targetJobId = targetStart.response.data.jobId;
    const running = await waitForStatus(
      call,
      targetRoot.id,
      (status) =>
        status?.jobId === targetJobId &&
        status.state === "running" &&
        typeof status.runId === "string" &&
        status.runId.length > 0,
      30000,
      "target-running",
    );
    if (!running) throw new Error("target never reached running");
    targetRunId = running.runId;
    log({
      kind: "target-running-confirmed",
      method: "native",
      status: compactStatus(running),
    });

    // Both requests happen while targetRoot owns the worker lease. They must
    // be queued with null run IDs before either root is disabled/removed.
    const disableStart = await invoke(call, "scan_now", {
      schemaVersion: 1,
      rootId: disableRoot.id,
    });
    const removeStart = await invoke(call, "scan_now", {
      schemaVersion: 1,
      rootId: removeRoot.id,
    });
    log({
      kind: "queued-scan-now",
      method: "native",
      disable: disableStart,
      remove: removeStart,
    });
    const disableQueued = await statusFor(call, disableRoot.id);
    const removeQueued = await statusFor(call, removeRoot.id);
    log({
      kind: "queued-before-mutation",
      method: "native",
      disable: compactStatus(disableQueued),
      remove: compactStatus(removeQueued),
      target: compactStatus(await statusFor(call, targetRoot.id)),
    });
    if (
      disableStart.response?.data?.runId !== null ||
      removeStart.response?.data?.runId !== null ||
      disableQueued?.state !== "queued" ||
      removeQueued?.state !== "queued"
    ) {
      throw new Error("the installed run did not capture both queued states");
    }
    await libraryUiSnapshot(call, "queued-before-mutation");

    const disabled = await invoke(call, "set_scan_root_enabled", {
      schemaVersion: 1,
      id: disableRoot.id,
      enabled: false,
    });
    const disabledStatus = await statusFor(call, disableRoot.id);
    log({
      kind: "queued-disable",
      method: "native",
      result: disabled,
      status: compactStatus(disabledStatus),
    });
    if (
      !disabled.ok ||
      disabled.response.status !== "ok" ||
      disabled.response.data.enabled !== false ||
      disabledStatus?.state !== "cancelled" ||
      disabledStatus.jobId !== disableStart.response.data.jobId
    ) {
      throw new Error("queued disable did not cancel the queued job in place");
    }
    await libraryUiSnapshot(call, "after-queued-disable");

    const removed = await invoke(call, "remove_scan_root", {
      schemaVersion: 1,
      id: removeRoot.id,
    });
    const rootsAfterRemove = await listRoots(call);
    const statusesAfterRemove = await listStatuses(call);
    const removedScan = await invoke(call, "scan_now", {
      schemaVersion: 1,
      rootId: removeRoot.id,
    });
    log({
      kind: "queued-remove",
      method: "native",
      result: removed,
      rootsRemaining: rootsAfterRemove.map((root) => ({
        id: root.id,
        enabled: root.enabled,
      })),
      removedStatusPresent: statusesAfterRemove.some(
        (status) => status.root.id === removeRoot.id,
      ),
      scanAfterRemove: removedScan,
    });
    if (
      !removed.ok ||
      removed.response.status !== "ok" ||
      rootsAfterRemove.some((root) => root.id === removeRoot.id) ||
      statusesAfterRemove.some((status) => status.root.id === removeRoot.id)
    ) {
      throw new Error("queued remove did not detach the root");
    }

    // Close only the CDP connection; this does not send a product shutdown
    // signal. The next operation is the exact-PID hard kill while targetRoot
    // is still running, followed immediately by a durable-state copy.
    websocket.close();
    const killStartedAt = new Date().toISOString();
    execFileSync("taskkill", ["/F", "/PID", String(firstApp.child.pid)], {
      stdio: "ignore",
    });
    const killedAt = new Date().toISOString();
    const durableDirectory = path.join(journeyRoot, "hard-kill-before-restart");
    const copied = copyDurableState(durableDirectory);
    const copiedAt = new Date().toISOString();
    log({
      kind: "hard-kill-before-restart",
      method: "native",
      pid: firstApp.child.pid,
      targetJobId,
      targetRunId,
      killStartedAt,
      killedAt,
      copiedAt,
      copiedMillisecondsAfterKill: Date.parse(copiedAt) - Date.parse(killedAt),
      processAliveAfterCopy: isProcessAlive(firstApp.child.pid),
      copied,
      durableDirectory: "hard-kill-before-restart",
    });
    if (isProcessAlive(firstApp.child.pid)) {
      throw new Error("the exact target app process survived taskkill");
    }

    secondApp = await launchApp();
    await sleep(3500);
    const restartedTarget = await discover(secondApp.port);
    secondConnection = await connect(restartedTarget);
    const { websocket: restartedWebsocket, call: restartedCall } =
      secondConnection;
    log({
      kind: "restarted-ready",
      method: "UI",
      url: restartedTarget.url,
      title: restartedTarget.title,
    });
    let firstAfterRestart = true;
    const recovered = await waitForStatus(
      restartedCall,
      targetRoot.id,
      (status) => {
        if (firstAfterRestart) {
          log({
            kind: "first-status-after-restart",
            method: "native",
            status: compactStatus(status),
          });
          firstAfterRestart = false;
        }
        return status?.jobId === targetJobId && status.state === "completed";
      },
      120000,
      "target-after-restart",
    );
    const pageAfterRestart = await getPage(restartedCall, targetRoot.id);
    const rootsAfterRestart = await listRoots(restartedCall);
    const disabledAfterRestart = rootsAfterRestart.find(
      (root) => root.id === disableRoot.id,
    );
    log({
      kind: "restart-result",
      method: "native",
      status: compactStatus(recovered),
      sameJob: recovered?.jobId === targetJobId,
      freshRun: recovered?.runId !== targetRunId,
      page: pageSummary(pageAfterRestart),
      disabledRoot: disabledAfterRestart
        ? { id: disabledAfterRestart.id, enabled: disabledAfterRestart.enabled }
        : null,
      removedRootPresent: rootsAfterRestart.some(
        (root) => root.id === removeRoot.id,
      ),
    });
    if (
      !recovered ||
      recovered.jobId !== targetJobId ||
      recovered.state !== "completed" ||
      recovered.runId === targetRunId ||
      !disabledAfterRestart ||
      disabledAfterRestart.enabled ||
      rootsAfterRestart.some((root) => root.id === removeRoot.id)
    ) {
      throw new Error(
        "same-job restart recovery or queued-root persistence failed",
      );
    }
    await libraryUiSnapshot(restartedCall, "after-restart");
    restartedWebsocket.close();
    log({ kind: "driver-result", pass: true });
  } finally {
    try {
      firstConnection?.websocket?.close();
    } catch {
      // Cleanup is best effort after an already-closed hard-killed process.
    }
    try {
      secondConnection?.websocket?.close();
    } catch {
      // Cleanup is best effort.
    }
    if (secondApp?.child && isProcessAlive(secondApp.child.pid)) {
      await stopExactProcess(secondApp.child, false);
    }
    if (firstApp?.child && isProcessAlive(firstApp.child.pid)) {
      await stopExactProcess(firstApp.child, true);
    }
  }
}

await main();
