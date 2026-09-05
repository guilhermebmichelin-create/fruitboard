import process from "node:process";
import {
  APP_READINESS_TIMEOUT_MS,
  CDP_EVALUATION_TIMEOUT_MS,
  SHELL_READINESS_EXPRESSION,
  WEBVIEW_READINESS_TIMEOUT_MS,
  assessShellReadiness,
  selectAppTarget,
} from "./lib/webview-probe.mjs";

const port = Number.parseInt(process.argv[2] ?? "", 10);
if (!Number.isInteger(port) || port < 1 || port > 65_535) {
  console.error("Usage: node scripts/probe-webview-audio.mjs <debug-port>");
  process.exit(2);
}

async function readTargets() {
  const response = await fetch(`http://127.0.0.1:${port}/json/list`);
  return response.json();
}

// Phase 1 — WebView readiness: a debuggable target served from the packaged
// app origin appears. Blank, loading, or foreign pages never satisfy this.
const webviewStarted = Date.now();
let appTarget;
for (;;) {
  try {
    const selection = selectAppTarget(await readTargets());
    if (selection.ok) {
      appTarget = selection.target;
      break;
    }
    if (Date.now() - webviewStarted >= WEBVIEW_READINESS_TIMEOUT_MS) {
      console.error(
        `The packaged app target did not appear within the bounded timeout: ${selection.reason}.`,
      );
      process.exit(1);
    }
  } catch {
    if (Date.now() - webviewStarted >= WEBVIEW_READINESS_TIMEOUT_MS) {
      console.error(
        "The packaged WebView2 page did not become ready within the bounded timeout.",
      );
      process.exit(1);
    }
    // WebView2 starts asynchronously; retry until the bounded deadline.
  }
  await new Promise((resolve) => setTimeout(resolve, 50));
}
const webviewReadyMs = Date.now() - webviewStarted;

const socket = new WebSocket(appTarget.webSocketDebuggerUrl);
let nextId = 0;
const pending = new Map();
const socketReady = new Promise((resolve, reject) => {
  const timer = setTimeout(
    () => reject(new Error("CDP connection timed out")),
    CDP_EVALUATION_TIMEOUT_MS,
  );
  socket.addEventListener("open", () => {
    clearTimeout(timer);
    resolve();
  });
  socket.addEventListener("error", () => {
    clearTimeout(timer);
    reject(new Error("CDP connection failed"));
  });
});
socket.addEventListener("message", (event) => {
  const message = JSON.parse(String(event.data));
  const waiter = pending.get(message.id);
  if (waiter) {
    pending.delete(message.id);
    clearTimeout(waiter.timer);
    waiter.resolve(message);
  }
});
socket.addEventListener("error", () => {
  for (const waiter of pending.values()) {
    clearTimeout(waiter.timer);
    waiter.reject(new Error("CDP connection failed"));
  }
  pending.clear();
});

function evaluate(expression) {
  return new Promise((resolve, reject) => {
    const id = (nextId += 1);
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error("CDP evaluation timed out"));
    }, CDP_EVALUATION_TIMEOUT_MS);
    pending.set(id, { resolve, reject, timer });
    socket.send(
      JSON.stringify({
        id,
        method: "Runtime.evaluate",
        params: { expression, returnByValue: true },
      }),
    );
  });
}

try {
  await socketReady;

  // Phase 2 — application readiness: the shell renders a usable marker
  // (document title, primary navigation, page heading) with no loading or
  // error state. Kept distinct from WebView readiness above.
  const appStarted = Date.now();
  let shell;
  for (;;) {
    const message = await evaluate(SHELL_READINESS_EXPRESSION);
    const assessment = assessShellReadiness(message.result?.result?.value);
    if (assessment.ok) {
      shell = message.result.result.value;
      break;
    }
    if (Date.now() - appStarted >= APP_READINESS_TIMEOUT_MS) {
      console.error(
        `The packaged app shell did not render within the bounded timeout: ${assessment.reasons.join(", ")}.`,
      );
      process.exit(1);
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  const appReadyMs = Date.now() - appStarted;

  const audioExpression = String.raw`(() => {
    const audio = document.createElement("audio");
    return {
      audioCanPlayType: {
        aac: audio.canPlayType('audio/mp4; codecs="mp4a.40.2"'),
        flac: audio.canPlayType("audio/flac"),
        mp3: audio.canPlayType("audio/mpeg"),
        oggVorbis: audio.canPlayType('audio/ogg; codecs="vorbis"'),
        wavPcm: audio.canPlayType('audio/wav; codecs="1"'),
      },
      userAgent: navigator.userAgent.replace(/\([^)]*\)/gu, "(platform)"),
    };
  })()`;
  const audioMessage = await evaluate(audioExpression);
  const audio = audioMessage.result?.result?.value;
  if (!audio || audioMessage.result?.exceptionDetails) {
    console.error("The WebView2 audio capability probe failed closed.");
    process.exit(1);
  }

  for (const requiredFormat of ["wavPcm", "mp3", "flac"]) {
    if (!audio.audioCanPlayType?.[requiredFormat]) {
      console.error(
        "WebView2 did not advertise every required foundation audio format.",
      );
      process.exit(1);
    }
  }

  process.stdout.write(
    `${JSON.stringify({
      url: shell.pageUrl,
      title: shell.title,
      webviewReadyMs,
      appReadyMs,
      audioCanPlayType: audio.audioCanPlayType,
      userAgent: audio.userAgent,
    })}\n`,
  );
} catch (error) {
  console.error(
    error instanceof Error ? error.message : "The WebView2 probe failed.",
  );
  process.exit(1);
} finally {
  socket.close();
}
