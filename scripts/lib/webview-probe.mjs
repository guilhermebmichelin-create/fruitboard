// Shared WebView probe policy for the Windows packaging smoke.
//
// The packaged Fruitboard shell must be observed rendering before any audio
// capability signal is trusted. Selecting "any page target" would accept a
// synthetic about:blank document with an empty title, so target selection and
// shell readiness are explicit, bounded, and unit-tested in
// tests/webview-probe.test.mjs.

export const WEBVIEW_READINESS_TIMEOUT_MS = 60_000;
export const APP_READINESS_TIMEOUT_MS = 30_000;
export const CDP_EVALUATION_TIMEOUT_MS = 5_000;

// Origins that may serve the packaged Tauri shell. Anything else, including
// about:blank and remote https origins, must fail the probe closed.
export const APP_URL_PREFIXES = Object.freeze([
  "https://tauri.localhost",
  "http://tauri.localhost",
  "tauri://localhost",
]);

const APP_TITLE_SUFFIX = " · Fruitboard";

export function isAppTargetUrl(url) {
  return (
    typeof url === "string" &&
    APP_URL_PREFIXES.some((prefix) => url.startsWith(prefix))
  );
}

function summarizeTarget(target) {
  if (target === null || typeof target !== "object") {
    return { type: "unknown", url: "missing" };
  }
  const record = target;
  return {
    type: typeof record.type === "string" ? record.type : "unknown",
    url:
      typeof record.url === "string" && record.url !== ""
        ? record.url
        : "missing",
  };
}

export function classifyTarget(target) {
  if (target === null || typeof target !== "object") {
    return { kind: "unusable", reason: "non-object-target" };
  }
  const record = target;
  if (record.type !== "page") {
    return { kind: "unusable", reason: `unexpected-target-type` };
  }
  if (typeof record.webSocketDebuggerUrl !== "string") {
    return { kind: "unusable", reason: "target-not-debuggable" };
  }
  if (typeof record.url !== "string" || record.url === "") {
    return { kind: "blank", reason: "blank-or-missing-url" };
  }
  if (record.url === "about:blank") {
    return { kind: "blank", reason: "blank-or-missing-url" };
  }
  if (!isAppTargetUrl(record.url)) {
    return { kind: "unusable", reason: "unexpected-target-url" };
  }
  return { kind: "app", reason: "packaged-app-target" };
}

export function selectAppTarget(targets) {
  if (!Array.isArray(targets)) {
    return { ok: false, reason: "target-list-unavailable", observed: [] };
  }
  for (const candidate of targets) {
    if (classifyTarget(candidate).kind === "app") {
      return { ok: true, reason: "packaged-app-target", target: candidate };
    }
  }
  return {
    ok: false,
    reason: "no-app-target",
    observed: targets.map(summarizeTarget),
  };
}

// Evaluated inside the page. Reports the rendered shell markers the smoke
// requires: a real document title, the primary navigation with usable links,
// a page heading, and the absence of loading/error shell states.
export const SHELL_READINESS_EXPRESSION = String.raw`(() => {
  const navigation = document.querySelector('nav[aria-label="Primary"]');
  const heading = document.querySelector("h1");
  return {
    pageUrl: location.href,
    title: document.title,
    readyState: document.readyState,
    hasPrimaryNav: navigation !== null,
    navLinkCount:
      navigation === null
        ? 0
        : navigation.querySelectorAll("a[href]").length,
    hasHeading:
      heading !== null && heading.textContent.trim().length > 0,
    shellLoading:
      document.querySelector('[data-shell-state="loading"]') !== null,
    shellError:
      document.querySelector('[data-shell-state="error"]') !== null ||
      document.querySelector('[role="alert"]') !== null,
  };
})()`;

export function assessShellReadiness(snapshot) {
  const reasons = [];
  if (snapshot === null || typeof snapshot !== "object") {
    return { ok: false, reasons: ["shell-snapshot-unavailable"] };
  }
  const state = snapshot;
  if (
    typeof state.title !== "string" ||
    state.title === "" ||
    !state.title.endsWith(APP_TITLE_SUFFIX)
  ) {
    reasons.push("unexpected-document-title");
  }
  if (state.readyState !== "interactive" && state.readyState !== "complete") {
    reasons.push("document-not-ready");
  }
  if (state.hasPrimaryNav !== true || !(state.navLinkCount >= 1)) {
    reasons.push("primary-navigation-missing");
  }
  if (state.hasHeading !== true) {
    reasons.push("page-heading-missing");
  }
  if (state.shellLoading === true) {
    reasons.push("shell-still-loading");
  }
  if (state.shellError === true) {
    reasons.push("shell-error-state");
  }
  return { ok: reasons.length === 0, reasons };
}
