import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

// Installed-evidence driver for the P2-08 retry/presentation/a11y gaps.
//
// It drives the packaged Foundation Smoke executable through the installed
// WebView2 CDP endpoint and the native Tauri command surface. It records:
//   D2  cancel a running scan, prove the terminal presentation, then run the
//       actionable control the installed UI actually offers and prove it
//       reconciles (no non-actionable Retry is offered on current head).
//   D3  move a committed root away, observe the unavailable failure and the
//       durable automatic-retry chain exhaust, restore the root, then run the
//       actionable control and prove convergence.
//   #111  capture the typed scan-console presentation for every reachable
//       durable state (idle/queued/running/completed/cancelled/failed).
//   a11y  desktop + narrow screenshots, a keyboard focus trace, an AX-tree
//       summary, and a best-effort axe run for the console states.
//
// It is review evidence only. It does not change product code, budgets, or the
// S5 contract.

const journeyRoot = path.resolve(process.argv[2] ?? "");
const appPath = path.resolve(
  process.argv[3] ??
    path.join(journeyRoot, "Installed Fruitboard", "fruitboard-desktop.exe"),
);
const transcriptPath = path.join(journeyRoot, "p2o8-retry-state.jsonl");
const screenshotsDirectory = path.join(journeyRoot, "screenshots");
const dataDirectory = path.join(
  process.env.LOCALAPPDATA ?? "",
  "com.fruitboard.desktop.foundation-smoke",
);
const storageDirectory = path.join(dataDirectory, "storage");

if (!journeyRoot || !fs.existsSync(appPath)) {
  throw new Error(
    "usage: node p2o8-retry-state-capture.mjs <journey-root> [app-path]",
  );
}

fs.mkdirSync(journeyRoot, { recursive: true });
fs.mkdirSync(screenshotsDirectory, { recursive: true });

const repoRoot = fileURLToPath(new URL("../../../../", import.meta.url));
const require = createRequire(import.meta.url);
let axeSource = null;
for (const candidate of [
  () => require.resolve("axe-core/axe.min.js"),
  () => path.join(repoRoot, "node_modules", "axe-core", "axe.min.js"),
  () =>
    path.join(
      repoRoot,
      "apps",
      "client",
      "node_modules",
      "axe-core",
      "axe.min.js",
    ),
  () => {
    const pnpmRoot = path.join(repoRoot, "node_modules", ".pnpm");
    const match = fs
      .readdirSync(pnpmRoot)
      .find((entry) => entry.startsWith("axe-core@"));
    return path.join(
      pnpmRoot,
      match ?? "",
      "node_modules",
      "axe-core",
      "axe.min.js",
    );
  },
]) {
  try {
    const resolved = candidate();
    if (resolved && fs.existsSync(resolved)) {
      axeSource = fs.readFileSync(resolved, "utf8");
      break;
    }
  } catch {
    // Try the next candidate.
  }
}

function log(entry) {
  const line = JSON.stringify({ t: new Date().toISOString(), ...entry });
  fs.appendFileSync(transcriptPath, `${line}\n`, "utf8");
  console.log(line);
}

function sleep(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function sha256File(file) {
  return createHash("sha256").update(fs.readFileSync(file)).digest("hex");
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
            url.protocol === "http:" ||
            url.protocol === "https:" ||
            url.protocol === "tauri:"
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
      resolve({ websocket, send });
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

    function send(method, params = {}, timeoutMilliseconds = 30000) {
      return new Promise((resolveCall, rejectCall) => {
        const id = ++nextId;
        const timer = setTimeout(() => {
          pending.delete(id);
          rejectCall(new Error(`CDP ${method} timed out`));
        }, timeoutMilliseconds);
        pending.set(id, { resolve: resolveCall, reject: rejectCall, timer });
        websocket.send(JSON.stringify({ id, method, params }));
      });
    }
  });
}

async function call(send, expression, timeoutMilliseconds = 30000) {
  const message = await send(
    "Runtime.evaluate",
    { expression, returnByValue: true, awaitPromise: true },
    timeoutMilliseconds,
  );
  if (message.result?.exceptionDetails) {
    throw new Error(
      `DOM evaluation failed: ${JSON.stringify(message.result.exceptionDetails)}`,
    );
  }
  return message.result?.result?.value;
}

async function invoke(send, command, request) {
  const expression = `(async () => {
    try {
      const response = await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify({ request })});
      return { ok: true, response };
    } catch (error) {
      return { ok: false, error: String(error && error.message ? error.message : error) };
    }
  })()`;
  const value = await call(send, expression);
  if (!value) throw new Error(`invoke ${command} returned no value`);
  return value;
}

function compactStatus(status) {
  if (!status) return null;
  return {
    rootId: status.root?.id,
    displayName: status.root?.displayName,
    enabled: status.root?.enabled,
    availability: status.root?.availability,
    state: status.state,
    jobId: status.jobId,
    runId: status.runId,
    cancellationRequested: status.cancellationRequested,
    retryAvailable: status.retryAvailable,
    counters: status.counters,
    errorCode: status.errorCode,
  };
}

async function listStatuses(send) {
  const result = await invoke(send, "list_scan_statuses", {
    schemaVersion: 1,
  });
  if (!result.ok || result.response.status !== "ok") {
    throw new Error(`list_scan_statuses failed: ${JSON.stringify(result)}`);
  }
  return result.response.data;
}

async function listRoots(send) {
  const result = await invoke(send, "list_scan_roots", { schemaVersion: 1 });
  if (!result.ok || result.response.status !== "ok") {
    throw new Error(`list_scan_roots failed: ${JSON.stringify(result)}`);
  }
  return result.response.data;
}

async function statusFor(send, rootId) {
  return (
    (await listStatuses(send)).find((status) => status.root.id === rootId) ??
    null
  );
}

async function waitForAllSettled(send, rootIds, timeout) {
  const started = Date.now();
  let statuses = [];
  while (Date.now() - started < timeout) {
    statuses = await listStatuses(send);
    const active = statuses.filter(
      (status) =>
        rootIds.includes(status.root.id) &&
        (status.state === "queued" || status.state === "running"),
    );
    if (active.length === 0) return statuses;
    await sleep(200);
  }
  return statuses;
}

async function waitForStatus(send, rootId, predicate, timeout, label) {
  const started = Date.now();
  let lastKey = null;
  let lastStatus = null;
  while (Date.now() - started < timeout) {
    lastStatus = await statusFor(send, rootId);
    const compact = compactStatus(lastStatus);
    const key = JSON.stringify(compact);
    if (key !== lastKey) {
      log({ kind: "status-transition", label, status: compact });
      lastKey = key;
    }
    if (predicate(lastStatus)) return { status: lastStatus, timedOut: false };
    await sleep(100);
  }
  return { status: lastStatus, timedOut: true };
}

async function getPage(send, rootId, limit = 4) {
  return invoke(send, "get_library_page", {
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
      ?.slice(0, 4)
      .map((record) => `${record.fileName}:${record.presence}`),
  };
}

async function uiSnapshot(send, label) {
  await call(
    send,
    `(() => { location.hash = "#/library"; return location.hash; })()`,
  );
  await sleep(300);
  const snapshot = await call(
    send,
    `(() => ({
      adapter: document.querySelector("[data-review-adapter]")?.getAttribute("data-review-adapter") ?? null,
      libraryState: document.querySelector("[data-library-state]")?.getAttribute("data-library-state") ?? null,
      progress: [...document.querySelectorAll("[data-scan-progress]")].map((e) => e.getAttribute("data-scan-progress")),
      cards: [...document.querySelectorAll("[data-root-id]")].map((card) => ({
        rootId: card.getAttribute("data-root-id"),
        progress: card.querySelector("[data-scan-progress]")?.getAttribute("data-scan-progress") ?? null,
        text: (card.textContent ?? "").replace(/\\s+/g, " ").trim().slice(0, 420),
      })),
      controls: [...document.querySelectorAll("button")].map((button) => ({
        label: button.getAttribute("aria-label") ?? (button.textContent ?? "").trim().slice(0, 60),
        disabled: button.disabled,
      })),
      reviewBadge: document.querySelector(".library-review-badge") !== null,
      body: (document.body.innerText ?? "").replace(/\\s+/g, " ").trim().slice(0, 900),
    }))()`,
  );
  log({ kind: "ui-snapshot", label, method: "UI", snapshot });
  return snapshot;
}

async function screenshot(send, name, width, height) {
  if (width && height) {
    await send("Emulation.setDeviceMetricsOverride", {
      width,
      height,
      deviceScaleFactor: 1,
      mobile: width < 600,
    });
    await sleep(250);
  }
  const message = await send("Page.captureScreenshot", {
    format: "png",
    captureBeyondViewport: false,
  });
  const base64 = message.result?.data;
  if (width && height) {
    await send("Emulation.clearDeviceMetricsOverride");
  }
  if (!base64) return null;
  const file = path.join(screenshotsDirectory, `${name}.png`);
  fs.writeFileSync(file, Buffer.from(base64, "base64"));
  return {
    file: path.relative(journeyRoot, file),
    bytes: fs.statSync(file).size,
  };
}

function elementDescriptor() {
  return `(() => {
    const active = document.activeElement;
    if (!active) return null;
    const tag = active.tagName.toLowerCase();
    return {
      tag,
      role: active.getAttribute("role"),
      label: active.getAttribute("aria-label"),
      text: (active.textContent ?? "").replace(/\\s+/g, " ").trim().slice(0, 80),
      id: active.id || null,
    };
  })()`;
}

async function keyboardTrace(send, label, steps = 12) {
  await call(
    send,
    `(() => { document.body.focus(); return document.activeElement ? document.activeElement.tagName : null; })()`,
  );
  const trace = [];
  for (let index = 0; index < steps; index += 1) {
    await send("Input.dispatchKeyEvent", {
      type: "rawKeyDown",
      windowsVirtualKeyCode: 9,
      nativeVirtualKeyCode: 9,
      key: "Tab",
      code: "Tab",
    });
    await send("Input.dispatchKeyEvent", {
      type: "keyUp",
      windowsVirtualKeyCode: 9,
      nativeVirtualKeyCode: 9,
      key: "Tab",
      code: "Tab",
    });
    await sleep(120);
    trace.push(await call(send, elementDescriptor()));
  }
  log({ kind: "keyboard-trace", label, method: "UI", trace });
  return trace;
}

async function accessibilityTree(send, label) {
  await send("Accessibility.enable");
  const message = await send("Accessibility.getFullAXTree");
  const nodes = (message.result?.nodes ?? [])
    .filter((node) => node.role?.value && node.role.value !== "generic")
    .map((node) => ({
      role: node.role?.value,
      name: node.name?.value ?? "",
      disabled:
        node.properties?.find((p) => p.name === "disabled")?.value?.value ??
        null,
    }))
    .filter((node) =>
      [
        "button",
        "link",
        "heading",
        "status",
        "alert",
        "list",
        "listitem",
        "tab",
        "textbox",
      ].includes(node.role),
    );
  const summary = {
    count: nodes.length,
    controls: nodes.filter((node) =>
      ["button", "link", "textbox", "tab"].includes(node.role),
    ),
    announcements: nodes.filter((node) =>
      ["status", "alert", "heading"].includes(node.role),
    ),
  };
  log({ kind: "accessibility-tree", label, method: "AX", summary });
  return summary;
}

async function runAxe(send, label) {
  if (!axeSource) {
    log({
      kind: "axe",
      label,
      method: "AX",
      ran: false,
      reason: "axe-core not resolvable",
    });
    return null;
  }
  const expression = `(async () => {
    try {
      ${axeSource}
      const results = await axe.run(document, { resultTypes: ["violations"] });
      return {
        ran: true,
        violations: results.violations.map((violation) => ({
          id: violation.id,
          impact: violation.impact,
          nodes: violation.nodes.length,
        })),
      };
    } catch (error) {
      return { ran: false, reason: String(error && error.message ? error.message : error) };
    }
  })()`;
  try {
    const result = await call(send, expression, 60000);
    log({ kind: "axe", label, method: "AX", ...result });
    return result;
  } catch (error) {
    log({
      kind: "axe",
      label,
      method: "AX",
      ran: false,
      reason: String(error),
    });
    return null;
  }
}

async function captureState(send, label, options = {}) {
  const ui = await uiSnapshot(send, label);
  const desktop = await screenshot(send, `${label}-desktop`);
  const narrow = await screenshot(send, `${label}-narrow`, 390, 844);
  log({ kind: "screenshots", label, desktop, narrow });
  if (options.keyboard) await keyboardTrace(send, label, options.keyboard);
  if (options.a11y) {
    await accessibilityTree(send, label);
    await runAxe(send, label);
  }
  return ui;
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
    return output.includes(`,"${pid}",`);
  } catch {
    return false;
  }
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
  const started = Date.now();
  while (child.exitCode === null && Date.now() - started < 5000) {
    await sleep(100);
  }
  if (!force && isProcessAlive(child.pid)) await stopExactProcess(child, true);
}

async function copyDurableState(destinationDirectory) {
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
      sha256: sha256File(destination),
    });
  }
  return copied;
}

async function clickByAriaLabelPrefix(send, prefix) {
  return call(
    send,
    `(() => {
      const button = [...document.querySelectorAll("button")].find((candidate) =>
        (candidate.getAttribute("aria-label") ?? "").startsWith(${JSON.stringify(prefix)})
      );
      if (!button) return { clicked: false, labels: [...document.querySelectorAll("button")].map((b) => b.getAttribute("aria-label")) };
      button.click();
      return { clicked: true, label: button.getAttribute("aria-label"), disabled: button.disabled };
    })()`,
  );
}

async function scanToCompleted(send, rootId, label, timeout = 240000) {
  const start = await invoke(send, "scan_now", { schemaVersion: 1, rootId });
  log({ kind: `${label}-scan-now`, method: "native", result: start });
  if (!start.ok || start.response.status !== "ok") {
    throw new Error(`${label} scan_now failed: ${JSON.stringify(start)}`);
  }
  const { status, timedOut } = await waitForStatus(
    send,
    rootId,
    (candidate) => candidate?.state === "completed",
    timeout,
    label,
  );
  if (timedOut || !status || status.state !== "completed") {
    throw new Error(
      `${label} did not complete: ${JSON.stringify(compactStatus(status))}`,
    );
  }
  return status;
}

async function main() {
  log({
    kind: "driver-start",
    driver: "p2o8-retry-state-capture.mjs",
    schemaVersion: 1,
    appPath,
    dataDirectory,
    axeResolved: axeSource !== null,
  });

  const fixtureRoot = path.join(journeyRoot, "fixture");
  const cancellationFixtureRoot = path.join(
    journeyRoot,
    "cancellation-fixture",
  );
  const primaryRootsDirectory = path.join(fixtureRoot, "roots");
  const leaves = fs
    .readdirSync(primaryRootsDirectory, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
  if (leaves.length < 2) {
    throw new Error("the synthetic fixture did not provide two sibling leaves");
  }
  const primaryLeaf = path.join(primaryRootsDirectory, leaves[0]);
  const idleLeaf = path.join(primaryRootsDirectory, leaves[1]);
  const longScanRoot = path.join(cancellationFixtureRoot, "roots");

  let app;
  let connection;
  const result = {
    pass: false,
    d2: null,
    d3: null,
    presentation: [],
    states: [],
  };
  try {
    app = await launchApp();
    await sleep(3500);
    const target = await discover(app.port);
    connection = await connect(target);
    const { websocket, send } = connection;
    log({
      kind: "app-ready",
      method: "UI",
      url: target.url,
      title: target.title,
    });

    const consoleState = await invoke(send, "get_scan_console_state", {
      schemaVersion: 1,
    });
    log({ kind: "scan-console-state", result: consoleState });
    if (
      !consoleState.ok ||
      consoleState.response.status !== "ok" ||
      !consoleState.response.data.enabled
    ) {
      throw new Error("the installed package did not enable scan-console");
    }
    const initialRoots = await listRoots(send);
    if (initialRoots.length !== 0) {
      throw new Error("the installed database was not isolated and empty");
    }

    async function addRoot(displayName, rootPath) {
      const response = await invoke(send, "add_scan_root", {
        schemaVersion: 1,
        displayName,
        path: rootPath,
      });
      log({
        kind: "root-added",
        displayName,
        relativePath: path.relative(journeyRoot, rootPath),
        result: response,
      });
      if (!response.ok || response.response.status !== "ok") {
        throw new Error(`failed to add ${displayName}`);
      }
      return response.response.data;
    }

    const longRoot = await addRoot("Long Scan Root", longScanRoot);
    const primaryRoot = await addRoot("Primary Root", primaryLeaf);
    const idleRoot = await addRoot("Idle Root", idleLeaf);
    fs.writeFileSync(
      path.join(journeyRoot, "root-ids.json"),
      JSON.stringify(
        {
          longRootId: longRoot.id,
          primaryRootId: primaryRoot.id,
          idleRootId: idleRoot.id,
          primaryLeafRelative: path.relative(journeyRoot, primaryLeaf),
          longScanRootRelative: path.relative(journeyRoot, longScanRoot),
        },
        null,
        2,
      ),
      "utf8",
    );

    // Startup recovery auto-enqueues enabled roots. Record the queued
    // presentation, then wait for the auto-recovery scans to settle.
    const startupStatuses = await listStatuses(send);
    log({
      kind: "startup-statuses",
      statuses: startupStatuses.map(compactStatus),
    });
    result.states.push({
      label: "queued-startup",
      status: startupStatuses.map(compactStatus),
    });
    const settledStatuses = await waitForAllSettled(
      send,
      [longRoot.id, primaryRoot.id, idleRoot.id],
      180000,
    );
    log({
      kind: "settled-statuses",
      statuses: settledStatuses.map(compactStatus),
    });

    // --- D2: cancel a running scan then use the actionable control ---------
    // The synthetic long scan completes in a few seconds, so the cancel must
    // be issued immediately after running is observed and the heavy capture
    // must wait until after the terminal state. Retry the race a few times.
    let d2Cancelled = { status: null, timedOut: true };
    let d2RunningObserved = null;
    for (let attempt = 1; attempt <= 4 && d2Cancelled.timedOut; attempt += 1) {
      const d2Start = await invoke(send, "scan_now", {
        schemaVersion: 1,
        rootId: longRoot.id,
      });
      log({ kind: "d2-scan-now", attempt, result: d2Start });
      if (!d2Start.ok || d2Start.response.status !== "ok") {
        throw new Error("D2 scan_now failed");
      }
      const d2JobId = d2Start.response.data.jobId;
      const d2Running = await waitForStatus(
        send,
        longRoot.id,
        (candidate) => candidate?.state === "running",
        30000,
        "d2-running",
      );
      if (d2Running.timedOut) throw new Error("D2 never reached running");
      d2RunningObserved = d2Running.status;
      if (attempt === 1) {
        result.states.push({
          label: "running",
          status: compactStatus(d2Running.status),
        });
      }
      const cancelResult = await invoke(send, "cancel_scan", {
        schemaVersion: 1,
        jobId: d2JobId,
      });
      log({ kind: "d2-cancel", attempt, result: cancelResult });
      const settled = await waitForStatus(
        send,
        longRoot.id,
        (candidate) =>
          candidate?.state === "cancelled" || candidate?.state === "completed",
        60000,
        "d2-cancelled",
      );
      if (settled.status?.state === "cancelled") {
        d2Cancelled = settled;
      } else {
        log({
          kind: "d2-cancel-race-lost",
          attempt,
          status: compactStatus(settled.status),
        });
      }
    }
    if (d2Cancelled.timedOut || !d2Cancelled.status) {
      throw new Error("D2 never reached cancelled after retries");
    }
    await captureState(send, "d2-cancelled", { keyboard: 10, a11y: true });
    result.states.push({
      label: "cancelled",
      status: compactStatus(d2Cancelled.status),
    });
    result.presentation.push({
      label: "cancelled",
      state: d2Cancelled.status?.state,
      errorCode: d2Cancelled.status?.errorCode ?? null,
      retryAvailable: d2Cancelled.status?.retryAvailable ?? null,
    });

    // Current head must not offer a non-actionable Retry after cancellation.
    const d2Ui = await call(
      send,
      `(() => ({
        retryButtons: [...document.querySelectorAll("button")].map((b) => b.getAttribute("aria-label")).filter((l) => l && l.startsWith("Retry scan ")),
        scanNowButtons: [...document.querySelectorAll("button")].map((b) => b.getAttribute("aria-label")).filter((l) => l && l.startsWith("Scan now ")),
      }))()`,
    );
    log({ kind: "d2-actions", method: "UI", ui: d2Ui });
    if ((d2Cancelled.status?.retryAvailable ?? false) !== false) {
      throw new Error("D2 cancellation unexpectedly offered Retry");
    }
    const d2Click = await clickByAriaLabelPrefix(
      send,
      "Scan now Long Scan Root",
    );
    log({ kind: "d2-scan-now-click", result: d2Click });
    if (!d2Click.clicked || d2Click.disabled) {
      throw new Error("D2 UI Scan now control was not actionable");
    }
    const d2Completed = await waitForStatus(
      send,
      longRoot.id,
      (candidate) => candidate?.state === "completed",
      300000,
      "d2-completed",
    );
    if (d2Completed.timedOut) throw new Error("D2 Scan now did not converge");
    await captureState(send, "d2-completed", { keyboard: 10, a11y: true });
    result.d2 = {
      cancelledState: compactStatus(d2Cancelled.status),
      retryOfferedAfterCancel:
        (d2Cancelled.status?.retryAvailable ?? false) !== false,
      actionableControl: d2Click.label ?? null,
      convergedState: compactStatus(d2Completed.status),
      converged: true,
    };

    // --- D3: unavailable root, exhausted retry chain, restore ---------------
    await scanToCompleted(send, primaryRoot.id, "d3-baseline");
    const committedPage = await getPage(send, primaryRoot.id, 200);
    log({ kind: "d3-baseline-page", page: pageSummary(committedPage) });

    const movedLeaf = path.join(primaryRootsDirectory, `${leaves[0]}__moved`);
    fs.renameSync(primaryLeaf, movedLeaf);
    log({
      kind: "d3-root-moved",
      from: path.relative(journeyRoot, primaryLeaf),
      to: path.relative(journeyRoot, movedLeaf),
    });

    // Deterministically request a scan of the now-unavailable root. If the
    // watcher already enqueued one, the native call coalesces safely.
    const d3Nudge = await invoke(send, "scan_now", {
      schemaVersion: 1,
      rootId: primaryRoot.id,
    });
    log({ kind: "d3-unavailable-scan-now", result: d3Nudge });

    const d3Failure = await waitForStatus(
      send,
      primaryRoot.id,
      (candidate) =>
        candidate?.state === "failed" && candidate?.retryAvailable === false,
      120000,
      "d3-exhausted-unavailable",
    );
    if (!d3Failure.status || d3Failure.status.state !== "failed") {
      throw new Error(
        `D3 did not reach an exhausted failed state: ${JSON.stringify(compactStatus(d3Failure.status))}`,
      );
    }
    await captureState(send, "d3-failed-unavailable", {
      keyboard: 10,
      a11y: true,
    });
    result.states.push({
      label: "failed-unavailable",
      status: compactStatus(d3Failure.status),
    });
    result.presentation.push({
      label: "failed-unavailable",
      state: d3Failure.status?.state,
      errorCode: d3Failure.status?.errorCode ?? null,
      retryAvailable: d3Failure.status?.retryAvailable ?? null,
    });
    const d3Actions = await call(
      send,
      `(() => ({
        retryButtons: [...document.querySelectorAll("button")].map((b) => b.getAttribute("aria-label")).filter((l) => l && l.startsWith("Retry scan ")),
        scanNowButtons: [...document.querySelectorAll("button")].map((b) => b.getAttribute("aria-label")).filter((l) => l && l.startsWith("Scan now ")),
      }))()`,
    );
    log({ kind: "d3-actions", method: "UI", ui: d3Actions });

    fs.renameSync(movedLeaf, primaryLeaf);
    log({
      kind: "d3-root-restored",
      path: path.relative(journeyRoot, primaryLeaf),
    });
    const d3Click = await clickByAriaLabelPrefix(send, "Scan now Primary Root");
    log({ kind: "d3-scan-now-click", result: d3Click });
    if (!d3Click.clicked || d3Click.disabled) {
      throw new Error(
        "D3 UI Scan now control was not actionable after restore",
      );
    }
    const d3Completed = await waitForStatus(
      send,
      primaryRoot.id,
      (candidate) => candidate?.state === "completed",
      300000,
      "d3-completed-after-restore",
    );
    if (d3Completed.timedOut)
      throw new Error("D3 did not converge after restore");
    await captureState(send, "d3-completed-after-restore");
    result.d3 = {
      exhaustedState: compactStatus(d3Failure.status),
      retryOfferedAfterExhaustion:
        (d3Failure.status?.retryAvailable ?? false) !== false,
      actionableControl: d3Click.label ?? null,
      convergedState: compactStatus(d3Completed.status),
      converged: true,
    };

    // --- queued presentation: queue behind a running long scan --------------
    await invoke(send, "scan_now", { schemaVersion: 1, rootId: longRoot.id });
    const queuedRunning = await waitForStatus(
      send,
      longRoot.id,
      (candidate) => candidate?.state === "running",
      30000,
      "queued-running-owner",
    );
    if (!queuedRunning.timedOut) {
      await screenshot(send, "running-desktop");
      result.states.push({
        label: "running",
        status: compactStatus(queuedRunning.status),
      });
      const queueResponse = await invoke(send, "scan_now", {
        schemaVersion: 1,
        rootId: primaryRoot.id,
      });
      const queuedStatus = await statusFor(send, primaryRoot.id);
      log({
        kind: "queued-presentation",
        queueResponse,
        status: compactStatus(queuedStatus),
      });
      if (queuedStatus?.state === "queued") {
        await captureState(send, "queued");
        result.states.push({
          label: "queued",
          status: compactStatus(queuedStatus),
        });
        result.presentation.push({
          label: "queued",
          state: queuedStatus.state,
          errorCode: queuedStatus.errorCode ?? null,
          retryAvailable: queuedStatus.retryAvailable ?? null,
        });
      }
      // Stop the long scan so the run can finish cleanly.
      await invoke(send, "cancel_scan", {
        schemaVersion: 1,
        jobId: queuedRunning.status.jobId,
      });
    }

    // Durable snapshot for provenance.
    const durable = await copyDurableState(
      path.join(journeyRoot, "database-after-run"),
    );
    log({ kind: "durable-snapshot", files: durable });

    result.pass = true;
    websocket.close();
    log({ kind: "driver-result", pass: true, result });
  } catch (error) {
    log({ kind: "driver-error", error: String(error?.stack ?? error) });
    result.error = String(error?.message ?? error);
    log({ kind: "driver-result", pass: false, result });
    process.exitCode = 1;
  } finally {
    try {
      connection?.websocket?.close();
    } catch {
      // Best-effort cleanup.
    }
    if (app?.child && isProcessAlive(app.child.pid)) {
      await stopExactProcess(app.child, false);
    }
    fs.writeFileSync(
      path.join(journeyRoot, "driver-result.json"),
      JSON.stringify(result, null, 2),
      "utf8",
    );
  }
}

await main();
