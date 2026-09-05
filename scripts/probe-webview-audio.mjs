import process from "node:process";

const port = Number.parseInt(process.argv[2] ?? "", 10);
if (!Number.isInteger(port) || port < 1 || port > 65_535) {
  console.error("Usage: node scripts/probe-webview-audio.mjs <debug-port>");
  process.exit(2);
}

const deadline = Date.now() + 15_000;
let target;
while (Date.now() < deadline) {
  try {
    const targets = await fetch(`http://127.0.0.1:${port}/json/list`).then(
      (response) => response.json(),
    );
    target = targets.find(
      (candidate) =>
        candidate.type === "page" &&
        typeof candidate.webSocketDebuggerUrl === "string",
    );
    if (target) break;
  } catch {
    // WebView2 starts asynchronously; retry until the bounded deadline.
  }
  await new Promise((resolve) => setTimeout(resolve, 50));
}

if (!target) {
  console.error(
    "The packaged WebView2 page did not become ready in 15 seconds.",
  );
  process.exit(1);
}

const expression = String.raw`(() => {
  const audio = document.createElement("audio");
  return {
    audioCanPlayType: {
      aac: audio.canPlayType('audio/mp4; codecs="mp4a.40.2"'),
      flac: audio.canPlayType("audio/flac"),
      mp3: audio.canPlayType("audio/mpeg"),
      oggVorbis: audio.canPlayType('audio/ogg; codecs="vorbis"'),
      wavPcm: audio.canPlayType('audio/wav; codecs="1"'),
    },
    documentTitle: document.title,
    userAgent: navigator.userAgent.replace(/\([^)]*\)/gu, "(platform)"),
  };
})()`;

const socket = new WebSocket(target.webSocketDebuggerUrl);
const response = await new Promise((resolve, reject) => {
  const timer = setTimeout(
    () => reject(new Error("CDP evaluation timed out")),
    5_000,
  );
  socket.addEventListener("open", () => {
    socket.send(
      JSON.stringify({
        id: 1,
        method: "Runtime.evaluate",
        params: { expression, returnByValue: true },
      }),
    );
  });
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(String(event.data));
    if (message.id === 1) {
      clearTimeout(timer);
      resolve(message);
    }
  });
  socket.addEventListener("error", () => {
    clearTimeout(timer);
    reject(new Error("CDP connection failed"));
  });
});
socket.close();

const value = response.result?.result?.value;
if (!value || response.result?.exceptionDetails) {
  console.error("The WebView2 audio capability probe failed closed.");
  process.exit(1);
}

for (const requiredFormat of ["wavPcm", "mp3", "flac"]) {
  if (!value.audioCanPlayType?.[requiredFormat]) {
    console.error(
      "WebView2 did not advertise every required foundation audio format.",
    );
    process.exit(1);
  }
}

process.stdout.write(`${JSON.stringify(value)}\n`);
