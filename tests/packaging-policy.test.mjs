import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const readRootFile = (path) =>
  readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

test("the package smoke is isolated, unsigned, and current-user only", () => {
  const config = JSON.parse(
    readRootFile("apps/desktop/src-tauri/tauri.package.conf.json"),
  );

  assert.equal(config.identifier, "com.fruitboard.desktop.foundation-smoke");
  assert.equal(config.bundle.active, true);
  assert.deepEqual(config.bundle.targets, ["nsis"]);
  assert.equal(config.bundle.createUpdaterArtifacts, false);
  assert.deepEqual(config.bundle.externalBin, [
    "binaries/fruitboard-sidecar-smoke",
  ]);
  assert.equal(config.bundle.windows.allowDowngrades, false);
  assert.deepEqual(config.bundle.windows.webviewInstallMode, {
    type: "downloadBootstrapper",
    silent: true,
  });
  assert.equal(config.bundle.windows.nsis.installMode, "currentUser");
  assert.equal(config.bundle.windows.signCommand, undefined);
  assert.equal(config.bundle.windows.certificateThumbprint, undefined);
});

test("the inert sidecar has bounded lifecycle modes and no product parser", () => {
  const manifest = readRootFile("crates/foundation-sidecar-smoke/Cargo.toml");
  const source = readRootFile("crates/foundation-sidecar-smoke/src/main.rs");
  const lifecycle = readRootFile(
    "crates/foundation-sidecar-smoke/tests/lifecycle.rs",
  );
  const nativeSmoke = readRootFile(
    "apps/desktop/src-tauri/src/foundation/packaging_smoke.rs",
  );

  assert.match(manifest, /publish = false/);
  assert.doesNotMatch(manifest, /^\[dependencies\]$/m);
  for (const mode of ["respond", "fail", "wait"]) {
    assert.match(source, new RegExp(`"${mode}"`));
    assert.match(nativeSmoke, new RegExp(`"${mode}"`));
  }
  assert.match(lifecycle, /spaces and Unicode/);
  assert.match(nativeSmoke, /\.sidecar\(PROBE_NAME\)/);
  assert.match(nativeSmoke, /\.kill\(\)/);
  assert.doesNotMatch(
    `${manifest}\n${source}\n${nativeSmoke}`,
    /(?:python|pyflp|scanner|watcher)/i,
  );
});

test("the renderer receives no process or shell permission", () => {
  const capability = readRootFile(
    "apps/desktop/src-tauri/capabilities/main.json",
  );
  const packageConfig = readRootFile(
    "apps/desktop/src-tauri/tauri.package.conf.json",
  );
  const desktopHost = readRootFile("apps/desktop/src-tauri/src/lib.rs");

  assert.doesNotMatch(capability, /(?:shell|process):/i);
  assert.doesNotMatch(packageConfig, /permissions|capabilities/);
  // The native folder picker runs behind the typed pick_scan_root command;
  // the renderer never invokes plugin dialogs directly.
  assert.match(desktopHost, /tauri_plugin_dialog::init\(\)/);
  assert.match(desktopHost, /blocking_pick_folder/);
  assert.doesNotMatch(capability, /dialog:/i);
});

test("the dialog dependency is exact, default-free, and locked", () => {
  const workspace = readRootFile("Cargo.toml");
  const lock = readRootFile("Cargo.lock");

  assert.match(
    workspace,
    /tauri-plugin-dialog = \{ version = "=2\.7\.3", default-features = false \}/,
  );
  for (const [name, version] of [
    ["tauri-plugin-dialog", "2.7.3"],
    ["rfd", "0.16.0"],
    ["tauri-plugin-fs", "2.5.2"],
  ]) {
    assert.match(lock, new RegExp(`name = "${name}"\\nversion = "${version}"`));
  }
});

test("the packaging workflow is bounded, read-only, and secret-free", () => {
  const workflow = readRootFile(
    ".github/workflows/windows-packaging-smoke.yml",
  );

  assert.match(workflow, /^  pull_request:$/m);
  assert.match(workflow, /^  schedule:$/m);
  assert.match(workflow, /^  workflow_dispatch:$/m);
  assert.match(workflow, /^permissions:\n  contents: read$/m);
  assert.match(workflow, /^  windows-packaging-smoke:$/m);
  assert.match(workflow, /pnpm\.cmd smoke:windows:foundation:hosted/);
  // The smoke is a required status check, so it must report on every pull
  // request: the trigger stays unfiltered. Every packaged input still
  // triggers the smoke, and docs-only pull requests get an honest packaging
  // result instead of a permanently missing required check.
  assert.doesNotMatch(workflow, /^\s+paths:$/m);
  assert.doesNotMatch(
    workflow,
    /pull_request_target|\bsecrets[.:]|upload-artifact|release|publish/i,
  );

  const actionReferences = [
    ...workflow.matchAll(/^\s*uses:\s*([^\s#]+)$/gm),
  ].map((match) => match[1]);
  assert.ok(actionReferences.length > 0);
  for (const reference of actionReferences) {
    assert.match(reference, /^[^@]+@[0-9a-f]{40}$/);
  }
  assert.equal(
    (workflow.match(/^\s+persist-credentials: false$/gm) ?? []).length,
    1,
  );
});

test("the smoke preserves data and records bounded platform evidence", () => {
  const launcher = readRootFile("scripts/run-windows-foundation-smoke.mjs");
  const audioProbe = readRootFile("scripts/probe-webview-audio.mjs");
  const script = readRootFile("scripts/windows-foundation-smoke.ps1");
  const packageJson = JSON.parse(readRootFile("package.json"));

  assert.match(
    packageJson.scripts["package:windows:smoke"],
    /prepare-foundation-sidecar/,
  );
  assert.match(
    packageJson.scripts["smoke:windows:foundation"],
    /run-windows-foundation-smoke\.mjs/,
  );
  assert.match(
    packageJson.scripts["smoke:windows:foundation:hosted"],
    /run-windows-foundation-smoke\.mjs -NonInteractive/,
  );
  assert.match(launcher, /spawnSync\(\s*"powershell\.exe"/);
  assert.match(launcher, /name\.toLowerCase\(\) !== "psmodulepath"/);
  assert.match(launcher, /windows-foundation-smoke\.ps1/);
  assert.match(audioProbe, /discoverAppTarget/);
  assert.match(audioProbe, /assessShellReadiness/);
  assert.match(audioProbe, /SHELL_READINESS_EXPRESSION/);
  assert.match(audioProbe, /WEBVIEW_READINESS_TIMEOUT_MS/);
  assert.match(audioProbe, /APP_READINESS_TIMEOUT_MS/);
  assert.match(audioProbe, /webviewReadyMs/);
  assert.match(audioProbe, /appReadyMs/);
  assert.match(audioProbe, /\{\s*signal,/);
  const probeLib = readRootFile("scripts/lib/webview-probe.mjs");
  assert.match(probeLib, /about:blank/);
  assert.match(probeLib, /tauri\.localhost/);
  assert.match(probeLib, /no-app-target/);
  assert.match(probeLib, /APP_ORIGINS/);
  assert.match(probeLib, /parsed\.username/);
  assert.match(probeLib, /unexpected-page-url/);
  assert.match(probeLib, /discoverAppTarget/);
  assert.match(probeLib, /AbortController/);
  assert.match(probeLib, /shell-still-loading/);
  assert.match(probeLib, /shell-error-state/);
  assert.match(probeLib, /WEBVIEW_READINESS_TIMEOUT_MS = 60_000/);
  assert.match(probeLib, /APP_READINESS_TIMEOUT_MS = 30_000/);
  assert.match(script, /firstUninstallPreservedDatabase = \$true/);
  assert.match(script, /reinstallRestoredStartupView = \$true/);
  assert.match(script, /secondUninstallPreservedDatabase = \$true/);
  assert.match(script, /Get-AuthenticodeSignature/);
  assert.match(
    script,
    /Import-Module Microsoft\.PowerShell\.Security -ErrorAction Stop/,
  );
  assert.match(script, /coldWebviewReadyMilliseconds/);
  assert.match(script, /coldAppReadyMilliseconds/);
  assert.match(script, /warmWebviewReadyMilliseconds/);
  assert.match(script, /warmAppReadyMilliseconds/);
  assert.match(script, /shellRendered = \$true/);
  assert.match(script, /schemaVersion = 2/);
  assert.match(script, /audioCanPlayType/);
  assert.match(script, /mode = "hosted-service-session"/);
  assert.match(script, /"not-probed-hosted-service-session"/);
  assert.match(script, /nativeSeedAndVerifyLaunches = \$true/);
  assert.doesNotMatch(script, /Remove-Item/);
});

test("the installed smoke deadline covers the bounded sidecar budget", () => {
  const script = readRootFile("scripts/windows-foundation-smoke.ps1");
  const nativeSmoke = readRootFile(
    "apps/desktop/src-tauri/src/foundation/packaging_smoke.rs",
  );

  const outer = script.match(/\$timeoutSeconds\s*=\s*(\d+)/);
  assert.ok(outer, "the outer smoke deadline must be an explicit budget");
  const outerMs = Number(outer[1]) * 1000;

  const durationMs = (name, source) => {
    const match = source.match(
      new RegExp(
        `${name}[^=]*=\\s*Duration::from_(secs|millis)\\(([\\d_]+)\\)`,
      ),
    );
    assert.ok(match, `${name} must stay a named bounded budget`);
    const value = Number(match[2].replaceAll("_", ""));
    return match[1] === "secs" ? value * 1000 : value;
  };
  const innerMs =
    durationMs("RESPOND_TIMEOUT", nativeSmoke) +
    durationMs("FAIL_TIMEOUT", nativeSmoke) +
    durationMs("WAIT_READY_TIMEOUT", nativeSmoke) +
    durationMs("WAIT_TERMINATE_TIMEOUT", nativeSmoke);

  // The outer harness deadline must cover the worst-case inner sidecar
  // budget plus startup, storage setup, evidence sync, and exit margin. The
  // previous 10s outer deadline left about 0.75s for all overhead, so
  // healthy slow runs were killed as timeouts.
  assert.ok(
    outerMs >= innerMs + 10_000,
    `outer ${outerMs}ms must cover inner ${innerMs}ms plus overhead margin`,
  );

  for (const token of [
    "mode=$Mode",
    "elapsedMs=",
    "hasExited=",
    "evidenceExists=",
    "evidenceBytes=",
    "evidenceSeenMs=",
  ]) {
    assert.ok(
      script.includes(token),
      `the timeout diagnostics must report ${token}`,
    );
  }
  for (const token of [
    "stages.jsonl",
    "schedule_start",
    "sidecar_respond_done",
    "sidecar_fail_done",
    "sidecar_wait_ready_done",
    "sidecar_wait_terminate_done",
    "evidence_written",
    "exit_requested",
    "respond_ms",
    "total_ms",
  ]) {
    assert.ok(
      nativeSmoke.includes(token),
      `the native smoke must record the bounded ${token} stage`,
    );
  }
  // Failure messages must name the smoke mode so seed and verify timeouts
  // are distinguishable without absolute paths or file contents.
  assert.match(script, /mode=\$Mode.*elapsedMs=/);
  assert.doesNotMatch(script, /Remove-Item/);
});
