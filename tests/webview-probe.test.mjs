import assert from "node:assert/strict";
import test from "node:test";
import {
  APP_ORIGINS,
  APP_READINESS_TIMEOUT_MS,
  SHELL_READINESS_EXPRESSION,
  WEBVIEW_READINESS_TIMEOUT_MS,
  assessShellReadiness,
  classifyTarget,
  discoverAppTarget,
  isAppTargetUrl,
  selectAppTarget,
} from "../scripts/lib/webview-probe.mjs";

const appTarget = (overrides = {}) => ({
  type: "page",
  url: "https://tauri.localhost/",
  webSocketDebuggerUrl: "ws://127.0.0.1:1/devtools/page/1",
  ...overrides,
});

const readyShell = (overrides = {}) => ({
  pageUrl: "https://tauri.localhost/",
  title: "Home · Fruitboard",
  readyState: "complete",
  hasPrimaryNav: true,
  navLinkCount: 4,
  hasHeading: true,
  shellLoading: false,
  shellError: false,
  ...overrides,
});

test("the probe only accepts targets served from the packaged app origin", () => {
  assert.equal(classifyTarget(appTarget()).kind, "app");
  assert.equal(
    classifyTarget(appTarget({ url: "tauri://localhost/" })).kind,
    "app",
  );
  assert.equal(
    classifyTarget(appTarget({ url: "http://tauri.localhost/" })).kind,
    "app",
  );
  assert.ok(APP_ORIGINS.length >= 1);
  for (const origin of APP_ORIGINS) {
    assert.match(origin.protocol, /^(https?|tauri):$/);
  }
});

test("origin validation parses URLs instead of matching prefixes", () => {
  // Regression: startsWith accepted foreign hosts with an app-origin prefix.
  assert.equal(isAppTargetUrl("https://tauri.localhost.example.com/"), false);
  assert.equal(
    classifyTarget(appTarget({ url: "https://tauri.localhost.example.com/" }))
      .kind,
    "unusable",
  );
  // Regression: a credential-bearing foreign URL smuggled the app host.
  assert.equal(isAppTargetUrl("https://tauri.localhost@evil.example/"), false);
  assert.equal(isAppTargetUrl("https://user:pass@tauri.localhost/"), false);
  // Unexpected ports, unknown schemes, and malformed URLs fail closed.
  assert.equal(isAppTargetUrl("https://tauri.localhost:8443/"), false);
  assert.equal(isAppTargetUrl("http://tauri.localhost:8080/"), false);
  assert.equal(isAppTargetUrl("tauri://localhost:1234/"), false);
  assert.equal(isAppTargetUrl("https://example.com/"), false);
  assert.equal(isAppTargetUrl("null"), false);
  assert.equal(isAppTargetUrl("not a url"), false);
  assert.equal(isAppTargetUrl(""), false);
  assert.equal(isAppTargetUrl(undefined), false);
  // Default ports are explicit; unrelated custom schemes never match.
  assert.equal(isAppTargetUrl("https://tauri.localhost:443/"), true);
  assert.equal(isAppTargetUrl("http://tauri.localhost:80/"), true);
  assert.equal(isAppTargetUrl("custom://localhost/"), false);
});

test("packaged routes, queries, and hash fragments remain accepted", () => {
  assert.equal(isAppTargetUrl("https://tauri.localhost/library"), true);
  assert.equal(
    isAppTargetUrl("https://tauri.localhost/board?view=kanban#card-1"),
    true,
  );
  assert.equal(isAppTargetUrl("tauri://localhost/preferences#startup"), true);
});

test("a foreign snapshot fails even with otherwise valid shell markers", () => {
  // Regression: navigation after target discovery bypassed validation.
  const navigatedAway = assessShellReadiness(
    readyShell({ pageUrl: "https://evil.example/" }),
  );
  assert.equal(navigatedAway.ok, false);
  assert.ok(navigatedAway.reasons.includes("unexpected-page-url"));
  const suffixed = assessShellReadiness(
    readyShell({ pageUrl: "https://tauri.localhost.example.com/" }),
  );
  assert.equal(suffixed.ok, false);
  assert.ok(suffixed.reasons.includes("unexpected-page-url"));
});

test("the probe rejects the synthetic blank-page acceptance", () => {
  // Regression: selecting any page target accepted about:blank with an empty
  // title, so a blank or broken renderer could pass the interactive smoke.
  assert.deepEqual(classifyTarget(appTarget({ url: "about:blank" })), {
    kind: "blank",
    reason: "blank-or-missing-url",
  });
  assert.equal(classifyTarget(appTarget({ url: "" })).kind, "blank");
  assert.equal(classifyTarget(appTarget({ url: undefined })).kind, "blank");
});

test("the probe ignores devtools, foreign, and non-debuggable targets", () => {
  assert.equal(
    classifyTarget(appTarget({ type: "service_worker" })).kind,
    "unusable",
  );
  assert.equal(
    classifyTarget(appTarget({ webSocketDebuggerUrl: undefined })).kind,
    "unusable",
  );
  assert.equal(
    classifyTarget(appTarget({ url: "https://example.com/" })).kind,
    "unusable",
  );
  assert.equal(
    classifyTarget(appTarget({ url: "chrome://gpu" })).kind,
    "unusable",
  );
  assert.equal(classifyTarget(null).kind, "unusable");
});

test("target selection prefers the app page and reports what it saw", () => {
  const selection = selectAppTarget([
    appTarget({
      url: "about:blank",
      webSocketDebuggerUrl: "ws://127.0.0.1:1/a",
    }),
    { type: "service_worker", url: "https://tauri.localhost/sw.js" },
    appTarget(),
  ]);
  assert.equal(selection.ok, true);
  assert.equal(selection.target.url, "https://tauri.localhost/");

  const missed = selectAppTarget([
    appTarget({ url: "about:blank" }),
    appTarget({ url: "https://example.com/" }),
  ]);
  assert.equal(missed.ok, false);
  assert.equal(missed.reason, "no-app-target");
  assert.deepEqual(missed.observed, [
    { type: "page", url: "about:blank" },
    { type: "page", url: "https://example.com/" },
  ]);

  assert.equal(selectAppTarget("not-a-list").ok, false);
  assert.equal(selectAppTarget([]).ok, false);
});

test("a rendered shell with navigation and heading is ready", () => {
  assert.deepEqual(assessShellReadiness(readyShell()), {
    ok: true,
    reasons: [],
  });
  assert.deepEqual(
    assessShellReadiness(
      readyShell({ title: "Library · Fruitboard", readyState: "interactive" }),
    ),
    { ok: true, reasons: [] },
  );
});

test("blank, loading, and error pages are not ready", () => {
  const blank = assessShellReadiness(
    readyShell({
      title: "",
      readyState: "loading",
      hasPrimaryNav: false,
      navLinkCount: 0,
      hasHeading: false,
      shellLoading: true,
    }),
  );
  assert.equal(blank.ok, false);
  assert.ok(blank.reasons.includes("unexpected-document-title"));
  assert.ok(blank.reasons.includes("primary-navigation-missing"));
  assert.ok(blank.reasons.includes("page-heading-missing"));
  assert.ok(blank.reasons.includes("shell-still-loading"));

  const error = assessShellReadiness(readyShell({ shellError: true }));
  assert.equal(error.ok, false);
  assert.deepEqual(error.reasons, ["shell-error-state"]);

  const wrongTitle = assessShellReadiness(readyShell({ title: "Example" }));
  assert.equal(wrongTitle.ok, false);
  assert.deepEqual(wrongTitle.reasons, ["unexpected-document-title"]);

  const navWithoutLinks = assessShellReadiness(readyShell({ navLinkCount: 0 }));
  assert.equal(navWithoutLinks.ok, false);
  assert.ok(navWithoutLinks.reasons.includes("primary-navigation-missing"));

  assert.equal(assessShellReadiness(null).ok, false);
});

test("the in-page readiness expression observes the shell contract", () => {
  assert.match(SHELL_READINESS_EXPRESSION, /nav\[aria-label="Primary"\]/);
  assert.match(SHELL_READINESS_EXPRESSION, /querySelector\("h1"\)/);
  assert.match(SHELL_READINESS_EXPRESSION, /data-shell-state="loading"/);
  assert.match(SHELL_READINESS_EXPRESSION, /data-shell-state="error"/);
  assert.match(SHELL_READINESS_EXPRESSION, /document\.title/);
  assert.match(SHELL_READINESS_EXPRESSION, /document\.readyState/);
});

test("a stalled discovery transport cannot exceed the deadline", async () => {
  // Regression: awaiting fetch and body consumption before checking elapsed
  // time let a never-resolving request block past the advertised deadline.
  const seen = [];
  const started = Date.now();
  const result = await discoverAppTarget({
    listTargets: (signal) => {
      seen.push(signal);
      return new Promise(() => {});
    },
    timeoutMs: 20,
    pollIntervalMs: 5,
  });
  const elapsed = Date.now() - started;
  assert.equal(result.ok, false);
  assert.equal(result.reason, "target-list-unavailable");
  assert.ok(
    elapsed < 2000,
    `discovery settled in ${elapsed} ms despite a stalled transport`,
  );
  assert.ok(seen.length >= 1);
  assert.ok(
    seen.every((signal) => signal.aborted),
    "every attempt signal is cancelled at the deadline",
  );
});

test("discovery retries an unavailable endpoint and then succeeds", async () => {
  let calls = 0;
  const result = await discoverAppTarget({
    listTargets: async () => {
      calls += 1;
      if (calls < 3) {
        throw new Error("connection refused");
      }
      return [appTarget()];
    },
    timeoutMs: 1000,
    pollIntervalMs: 1,
  });
  assert.equal(result.ok, true);
  assert.equal(result.target.url, "https://tauri.localhost/");
  assert.ok(typeof result.webviewReadyMs === "number");
  assert.equal(calls, 3);
});

test("discovery reports a fixed diagnostic when the endpoint never answers", async () => {
  const result = await discoverAppTarget({
    listTargets: async () => {
      throw new Error("connection refused");
    },
    timeoutMs: 20,
    pollIntervalMs: 5,
  });
  assert.deepEqual(result, {
    ok: false,
    reason: "target-list-unavailable",
    observed: [],
  });
});

test("discovery rejects non-list payloads and waits for the app target", async () => {
  const notAList = await discoverAppTarget({
    listTargets: async () => ({ targets: [] }),
    timeoutMs: 20,
    pollIntervalMs: 5,
  });
  assert.equal(notAList.ok, false);
  assert.equal(notAList.reason, "target-list-unavailable");

  const noApp = await discoverAppTarget({
    listTargets: async () => [appTarget({ url: "about:blank" })],
    timeoutMs: 20,
    pollIntervalMs: 5,
  });
  assert.equal(noApp.ok, false);
  assert.equal(noApp.reason, "no-app-target");
  assert.deepEqual(noApp.observed, [{ type: "page", url: "about:blank" }]);
});

test("readiness stays bounded with WebView and application phases distinct", () => {
  assert.equal(WEBVIEW_READINESS_TIMEOUT_MS, 60_000);
  assert.equal(APP_READINESS_TIMEOUT_MS, 30_000);
  assert.ok(APP_READINESS_TIMEOUT_MS <= WEBVIEW_READINESS_TIMEOUT_MS);
});
