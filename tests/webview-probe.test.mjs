import assert from "node:assert/strict";
import test from "node:test";
import {
  APP_READINESS_TIMEOUT_MS,
  APP_URL_PREFIXES,
  SHELL_READINESS_EXPRESSION,
  WEBVIEW_READINESS_TIMEOUT_MS,
  assessShellReadiness,
  classifyTarget,
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
  for (const prefix of APP_URL_PREFIXES) {
    assert.ok(isAppTargetUrl(`${prefix}/`));
  }
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

test("readiness stays bounded with WebView and application phases distinct", () => {
  assert.equal(WEBVIEW_READINESS_TIMEOUT_MS, 60_000);
  assert.equal(APP_READINESS_TIMEOUT_MS, 30_000);
  assert.ok(APP_READINESS_TIMEOUT_MS <= WEBVIEW_READINESS_TIMEOUT_MS);
});
