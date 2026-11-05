import assert from "node:assert/strict";
import { setTimeout as delay } from "node:timers/promises";

// Call only with the loopback CDP endpoint of an exclusively owned installed
// review app. Open a genuine extracted note sequence first. This changes only
// viewport/root-font emulation, never saved data, source authority or analysis.
const endpoint = new URL(process.argv[2] ?? "http://127.0.0.1:0");
assert.equal(endpoint.protocol, "http:");
assert.equal(endpoint.hostname, "127.0.0.1");
assert(Number(endpoint.port) > 0, "Supply the owned app's loopback CDP URL.");
const response = await fetch(new URL("/json/list", endpoint));
assert(response.ok, "The owned app's debug endpoint is unavailable.");
const pages = (await response.json()).filter(
  (target) =>
    target.type === "page" && target.url.startsWith("http://tauri.localhost/"),
);
assert.equal(pages.length, 1, "Expected one owned installed renderer.");
const socket = new WebSocket(pages[0].webSocketDebuggerUrl);
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
async function evaluate(expression) {
  const result = await send("Runtime.evaluate", {
    expression,
    returnByValue: true,
  });
  assert(!result.exceptionDetails, "The installed layout probe failed.");
  return result.result.value;
}
const originalFont = await evaluate("document.documentElement.style.fontSize");
const observations = [];
try {
  for (const width of [1280, 390]) {
    await send("Emulation.setDeviceMetricsOverride", {
      width,
      height: 1000,
      deviceScaleFactor: 1,
      mobile: false,
    });
    for (const fontSize of [16, 32]) {
      await evaluate(
        `document.documentElement.style.fontSize=${JSON.stringify(`${fontSize}px`)}`,
      );
      await delay(100);
      const value = await evaluate(`(() => {
        const notes = [...document.querySelectorAll('.project-note-sequence[open]')]
          .find(block => block.querySelector('.project-note-sequence__records li'));
        if (!notes) return null;
        const summary = notes.querySelector('summary');
        const box = summary.getBoundingClientRect();
        return {viewport: innerWidth, documentWidth: document.documentElement.scrollWidth,
          font: getComputedStyle(document.documentElement).fontSize,
          summaryWidth: box.width, summaryLeft: box.left, summaryRight: box.right,
          summaryLines: box.height / parseFloat(getComputedStyle(summary).lineHeight)};
      })()`);
      assert(value, "Open an extracted note sequence before this check.");
      assert.equal(value.font, `${fontSize}px`);
      assert.equal(value.viewport, width);
      assert(value.documentWidth <= width + 1, JSON.stringify(value));
      assert(value.summaryLeft >= 0 && value.summaryRight <= width + 1);
      if (width === 390 && fontSize === 32) {
        // Document overflow alone missed a column narrowed to about 98px.
        assert(value.summaryWidth >= width / 2, JSON.stringify(value));
        assert(value.summaryLines <= 4.1, JSON.stringify(value));
      }
      observations.push(value);
    }
  }
  console.log(JSON.stringify({ passed: true, observations }, null, 2));
} finally {
  await evaluate(
    `document.documentElement.style.fontSize=${JSON.stringify(originalFont)}`,
  );
  await send("Emulation.clearDeviceMetricsOverride");
  socket.close();
}
