import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
const [port, mode, fixtureRoot, evidence] = process.argv.slice(2);
const cases = [
  ["FIX-FL2026-MIN.flp", {state: "available", objectCount: 0, pointCount: 0}],
  ["FIX-FL2026-AUTO-2POINT.flp", {state: "available", objectCount: 1, pointCount: 2}],
  ["FIX-FL2024-A.flp", {state: "unverified_build"}],
];
async function waitFor(read, description, timeout = 45000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const value = await read();
    if (value) return value;
    await sleep(200);
  }
  throw new Error("Timed out: " + description);
}
const debuggerUrl = await waitFor(async () => {
  try {
    const response = await fetch("http://127.0.0.1:" + port + "/json/list", {signal: AbortSignal.timeout(1500)});
    if (!response.ok) return false;
    const targets = await response.json();
    return targets.find((target) => target.type === "page" && target.url.startsWith("http://tauri.localhost/"))?.webSocketDebuggerUrl;
  } catch {
    return false;
  }
}, "installed WebView");
assert.equal(new URL(debuggerUrl).port, port);
const socket = new WebSocket(debuggerUrl);
await new Promise((resolve, reject) => {
  socket.addEventListener("open", resolve, {once: true});
  socket.addEventListener("error", reject, {once: true});
});
const pending = new Map();
let sequence = 0;
socket.addEventListener("message", ({data}) => {
  const response = JSON.parse(data);
  pending.get(response.id)?.(response);
});
async function evaluate(expression) {
  const id = ++sequence;
  const result = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error("WebView evaluation timed out"));
    }, 15000);
    pending.set(id, (response) => {
      clearTimeout(timer);
      pending.delete(id);
      if (response.error) reject(new Error(JSON.stringify(response.error)));
      else resolve(response.result);
    });
    socket.send(JSON.stringify({id, method: "Runtime.evaluate", params: {expression, returnByValue: true, awaitPromise: true}}));
  });
  assert(!result.exceptionDetails, JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function invoke(command, args = {}) {
  const reply = await evaluate("window.__TAURI_INTERNALS__.invoke(" + JSON.stringify(command) + ",{request:" + JSON.stringify({schemaVersion: 1, ...args}) + "})");
  assert.equal(reply.schemaVersion, 1);
  assert.equal(reply.status, "ok", JSON.stringify(reply));
  return reply.data;
}
try {
  await waitFor(() => evaluate("Boolean(window.__TAURI_INTERNALS__?.invoke)"), "native bridge");
  const roots = await invoke("list_scan_roots");
  assert.equal(roots.length, mode === "first" ? 0 : 1);
  const root = mode === "first" ? await invoke("add_scan_root", {displayName: "Approved automation", path: fixtureRoot}) : roots[0];
  const rows = await waitFor(async () => {
    const page = await invoke("get_library_page", {rootId: root.id, limit: 100, cursor: null, snapshotId: null});
    return page.records.length === cases.length && page.records;
  }, "approved copies scanned", 90000);
  await invoke("set_startup_view", {startupView: "library"});
  await evaluate("location.hash='#/library'");
  await waitFor(() => evaluate("Boolean(document.querySelector('#library-root-select')?.querySelector('option[value=" + JSON.stringify(root.id) + "]'))"), "Library root option");
  await evaluate("(() => {const select = document.querySelector('#library-root-select'); select.value = " + JSON.stringify(root.id) + "; select.dispatchEvent(new Event('change', {bubbles: true}));})()");
  const snapshotPath = join(evidence, "snapshots.json");
  const snapshots = mode === "restart" ? JSON.parse(await readFile(snapshotPath, "utf8")) : {};
  for (const [file, expected] of cases) {
    const row = rows.find(({fileName}) => fileName === file);
    assert(row, file);
    assert.equal(row.presence, "present");
    const context = {rootId: row.rootId, locationId: row.locationId, expectedByteSize: row.byteSize, expectedModifiedAt: row.modifiedAt};
    const details = () => invoke("get_project_details", context);
    const initial = await details();
    if (mode === "first" && initial.state !== "available" && initial.analysisRequest?.state === "ready") {
      await invoke("request_project_analysis", {...context, requestKey: initial.analysisRequest.requestKey});
    }
    if (mode === "restart") assert.equal(initial.state, "available", file);
    const saved = await waitFor(async () => {
      const result = await details();
      return result.state === "available" && result;
    }, "saved details for " + file, 90000);
    assert.deepEqual(saved.automationCounts, expected, file);
    if (mode === "first") snapshots[file] = saved.snapshotId;
    else assert.equal(saved.snapshotId, snapshots[file], file);
    const button = "button[aria-label=\"Project details " + file + "\"]";
    const article = "article[aria-labelledby=\"library-record-" + row.locationId + "\"]";
    await waitFor(() => evaluate("Boolean(document.querySelector(" + JSON.stringify(button) + "))"), "project details button for " + file);
    await evaluate("document.querySelector(" + JSON.stringify(button) + ").click()");
    const section = article + " section[aria-label=\"Saved automation\"]";
    const rendered = await waitFor(() => evaluate("(() => {const node = document.querySelector(" + JSON.stringify(section) + "); return node && {text: node.innerText, values: [...node.querySelectorAll('dd')].map(cell => cell.textContent)};})()"), "rendered counts for " + file);
    if (expected.state === "available") {
      assert.deepEqual(rendered.values, [String(expected.objectCount), String(expected.pointCount)]);
      assert.match(rendered.text, /Saved Automation Clips/);
      assert.match(rendered.text, /Editable control points/);
    } else {
      assert.deepEqual(rendered.values, []);
      assert.match(rendered.text, /unavailable for this saved FL Studio build/);
    }
    console.log(mode, file, JSON.stringify(expected));
  }
  if (mode === "first") await writeFile(snapshotPath, JSON.stringify(snapshots), {flag: "wx"});
} finally {
  socket.close();
}
