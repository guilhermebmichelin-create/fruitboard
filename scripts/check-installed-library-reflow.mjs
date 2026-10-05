import assert from "node:assert/strict";
import { setTimeout as delay } from "node:timers/promises";

// Run against the loopback debug port of an exclusively owned installed review
// app whose Library contains at least two roots. This reads layout and changes
// only debugger emulation; it never scans, analyzes, or changes saved data.
const endpoint = new URL(process.argv[2] ?? "http://127.0.0.1:0");
assert.equal(endpoint.protocol, "http:");
assert.equal(endpoint.hostname, "127.0.0.1");
assert(Number(endpoint.port) > 0, "Supply the owned app's loopback CDP URL.");
const response = await fetch(new URL("/json/list", endpoint));
assert(response.ok, "The owned app's debug endpoint is unavailable.");
const targets = (await response.json()).filter(
  (target) =>
    target.type === "page" && target.url.startsWith("http://tauri.localhost/"),
);
assert.equal(targets.length, 1, "Expected one owned native app renderer.");
const socket = new WebSocket(targets[0].webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.addEventListener("open", resolve, { once: true });
  socket.addEventListener("error", reject, { once: true });
});
let nextId = 0;
const pending = new Map();
socket.addEventListener("message", ({ data }) => {
  const message = JSON.parse(data);
  const call = pending.get(message.id);
  if (!call) return;
  pending.delete(message.id);
  clearTimeout(call.timer);
  if (message.error) call.reject(new Error(message.error.message));
  else call.resolve(message.result);
});
function send(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++nextId;
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`CDP deadline: ${method}`));
    }, 10_000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params }));
  });
}
async function layout() {
  const result = await send("Runtime.evaluate", {
    expression: `(() => {
      const select = document.querySelector('#library-root-select');
      if (!select) return null;
      const box = select.getBoundingClientRect();
      return {font: getComputedStyle(document.documentElement).fontSize,
        viewport: innerWidth, documentWidth: document.documentElement.scrollWidth,
        rootCount: select.options.length, selectorLeft: box.left,
        selectorRight: box.right, selectorWidth: box.width};
    })()`,
    returnByValue: true,
  });
  assert(!result.exceptionDetails, "Layout probe could not execute.");
  return result.result.value;
}
async function ready(font) {
  for (let attempt = 0; attempt < 100; attempt++) {
    const value = await layout();
    if (value?.font === font) return value;
    await delay(100);
  }
  throw new Error(`Library did not load with ${font} text.`);
}
const observations = [];
try {
  const initial = await ready("16px");
  assert(initial.rootCount >= 2, "The regression requires at least two roots.");
  for (const width of [1280, 390]) {
    await send("Emulation.setDeviceMetricsOverride", {
      width,
      height: 1000,
      deviceScaleFactor: 1,
      mobile: false,
    });
    for (const [size, fixed] of [
      [16, 13],
      [32, 26],
    ]) {
      await send("Page.setFontSizes", {
        fontSizes: { standard: size, fixed },
      });
      // WebView2 applies the default text setting to a newly loaded document.
      await send("Page.reload");
      const value = await ready(`${size}px`);
      observations.push(value);
      assert.equal(value.viewport, width);
      assert(value.documentWidth <= width + 1, JSON.stringify(value));
      assert(value.selectorLeft >= 0, JSON.stringify(value));
      assert(value.selectorRight <= width + 1, JSON.stringify(value));
    }
  }
  console.log(JSON.stringify({ passed: true, observations }, null, 2));
} finally {
  await send("Page.setFontSizes", {
    fontSizes: { standard: 16, fixed: 13 },
  });
  await send("Emulation.clearDeviceMetricsOverride");
  await send("Page.reload");
  socket.close();
}
