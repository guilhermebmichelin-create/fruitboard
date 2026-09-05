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

  assert.doesNotMatch(capability, /(?:shell|process):/i);
  assert.doesNotMatch(packageConfig, /permissions|capabilities/);
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
  assert.match(audioProbe, /const readinessTimeoutMs = 60_000/);
  assert.match(script, /firstUninstallPreservedDatabase = \$true/);
  assert.match(script, /reinstallRestoredStartupView = \$true/);
  assert.match(script, /secondUninstallPreservedDatabase = \$true/);
  assert.match(script, /Get-AuthenticodeSignature/);
  assert.match(
    script,
    /Import-Module Microsoft\.PowerShell\.Security -ErrorAction Stop/,
  );
  assert.match(script, /coldReadyMilliseconds/);
  assert.match(script, /warmReadyMilliseconds/);
  assert.match(script, /audioCanPlayType/);
  assert.match(script, /mode = "hosted-service-session"/);
  assert.match(script, /"not-probed-hosted-service-session"/);
  assert.match(script, /nativeSeedAndVerifyLaunches = \$true/);
  assert.doesNotMatch(script, /Remove-Item/);
});
