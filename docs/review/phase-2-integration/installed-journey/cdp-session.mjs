import process from "node:process";

const port = Number.parseInt(process.argv[2] ?? "", 10);
if (!Number.isInteger(port) || port < 1 || port > 65_535) {
  console.error("Usage: node cdp-session.mjs <debug-port>");
  process.exit(2);
}

async function discoverTarget() {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/list`);
      const targets = await response.json();
      const target = targets.find(
        (candidate) =>
          candidate.type === "page" &&
          candidate.url?.startsWith("http://tauri.localhost"),
      );
      if (target?.webSocketDebuggerUrl) return target;
    } catch {
      // The packaged WebView may take a moment to expose its debug endpoint.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error("The installed Fruitboard WebView target was not found.");
}

const target = await discoverTarget();
const socket = new WebSocket(target.webSocketDebuggerUrl);
let nextId = 0;
const pending = new Map();

socket.addEventListener("message", (event) => {
  const message = JSON.parse(String(event.data));
  const waiter = pending.get(message.id);
  if (waiter === undefined) return;
  pending.delete(message.id);
  clearTimeout(waiter.timer);
  waiter.resolve(message);
});

function command(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++nextId;
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`CDP command timed out: ${method}`));
    }, 10_000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params }));
  });
}

await new Promise((resolve, reject) => {
  const timer = setTimeout(() => reject(new Error("CDP connection timed out")), 10_000);
  socket.addEventListener("open", () => {
    clearTimeout(timer);
    resolve();
  });
  socket.addEventListener("error", () => {
    clearTimeout(timer);
    reject(new Error("CDP connection failed"));
  });
});

async function evaluate(expression) {
  const response = await command("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  const result = response.result?.result;
  if (response.result?.exceptionDetails || result?.type === "undefined") {
    throw new Error(`Page evaluation failed: ${expression}`);
  }
  return result?.value;
}

const literal = (value) => JSON.stringify(value);

async function snapshot() {
  return evaluate(`(() => ({
    url: location.href,
    title: document.title,
    text: document.body.innerText,
    activeElement: document.activeElement?.outerHTML?.slice(0, 500) ?? null,
    controls: [...document.querySelectorAll("button, input, select, a")].map((element) => ({
      tag: element.tagName.toLowerCase(),
      text: element.innerText ?? "",
      ariaLabel: element.getAttribute("aria-label"),
      id: element.id || null,
      type: element.getAttribute("type"),
      value: element.value ?? null,
      checked: element.checked ?? null,
      disabled: element.disabled ?? false,
      href: element.getAttribute("href"),
    })),
  }))()`);
}

async function waitForText(text, timeoutMs = 10_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if ((await evaluate("document.body.innerText")).includes(text)) return true;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  return false;
}

async function waitForSelector(selector, timeoutMs = 10_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const present = await evaluate(
      `document.querySelector(${literal(selector)}) !== null`,
    );
    if (present) return true;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  return false;
}

async function click(selector) {
  return evaluate(`(() => {
    const element = document.querySelector(${literal(selector)});
    if (!(element instanceof HTMLElement)) throw new Error("missing clickable element");
    element.focus();
    element.click();
    return { selector: ${literal(selector)}, text: element.innerText ?? "" };
  })()`);
}

async function typeValue(selector, value) {
  return evaluate(`(() => {
    const element = document.querySelector(${literal(selector)});
    if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) {
      throw new Error("missing text input");
    }
    const descriptor = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value") ??
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value");
    descriptor?.set?.call(element, ${literal(value)});
    element.dispatchEvent(new Event("input", { bubbles: true }));
    element.dispatchEvent(new Event("change", { bubbles: true }));
    return { selector: ${literal(selector)}, value: element.value };
  })()`);
}

async function setSelect(selector, value) {
  return evaluate(`(() => {
    const element = document.querySelector(${literal(selector)});
    if (!(element instanceof HTMLSelectElement)) throw new Error("missing select");
    element.value = ${literal(value)};
    element.dispatchEvent(new Event("change", { bubbles: true }));
    return { selector: ${literal(selector)}, value: element.value };
  })()`);
}

async function setCheckbox(selector, checked) {
  return evaluate(`(() => {
    const element = document.querySelector(${literal(selector)});
    if (!(element instanceof HTMLInputElement) || element.type !== "checkbox") {
      throw new Error("missing checkbox");
    }
    if (element.checked !== ${checked}) element.click();
    return { selector: ${literal(selector)}, checked: element.checked };
  })()`);
}

async function key(selector, keyName) {
  return evaluate(`(() => {
    const element = document.querySelector(${literal(selector)});
    if (!(element instanceof HTMLElement)) throw new Error("missing keyboard target");
    element.focus();
    element.dispatchEvent(new KeyboardEvent("keydown", { key: ${literal(keyName)}, bubbles: true }));
    element.dispatchEvent(new KeyboardEvent("keyup", { key: ${literal(keyName)}, bubbles: true }));
    return { selector: ${literal(selector)}, key: ${literal(keyName)} };
  })()`);
}

async function run(request) {
  switch (request.op) {
    case "snapshot":
      return snapshot();
    case "eval":
      return evaluate(request.expression);
    case "waitText":
      return waitForText(request.text, request.timeoutMs ?? 10_000);
    case "waitSelector":
      return waitForSelector(request.selector, request.timeoutMs ?? 10_000);
    case "click":
      return click(request.selector);
    case "type":
      return typeValue(request.selector, request.value);
    case "select":
      return setSelect(request.selector, request.value);
    case "check":
      return setCheckbox(request.selector, request.checked);
    case "key":
      return key(request.selector, request.key);
    default:
      throw new Error(`Unknown operation: ${request.op}`);
  }
}

let input = "";
for await (const chunk of process.stdin) {
  input += chunk;
  const lines = input.split(/\r?\n/u);
  input = lines.pop() ?? "";
  for (const line of lines) {
    if (!line.trim()) continue;
    try {
      const result = await run(JSON.parse(line));
      process.stdout.write(`${JSON.stringify({ ok: true, result })}\n`);
    } catch (error) {
      process.stdout.write(
        `${JSON.stringify({
          ok: false,
          error: error instanceof Error ? error.message : String(error),
        })}\n`,
      );
    }
  }
}

socket.close();
