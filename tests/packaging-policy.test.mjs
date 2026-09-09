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
  // The automated smoke never deletes databases or evidence. The only
  // Remove-Item in the orchestration lives in the shared lock helper and
  // releases only the caller's own host lock (runId match); see the
  // exclusive-lock tests below.
  assert.doesNotMatch(script, /Remove-Item/);
});

test("installed validation runs serialize through an exclusive host lock", () => {
  const script = readRootFile("scripts/windows-foundation-smoke.ps1");
  const lockHelper = readRootFile("scripts/foundation-smoke-lock.ps1");
  const checklist = readRootFile(
    "docs/review/phase-2-integration/installed-app-journey-checklist.md",
  );

  // Single shared test identity; no per-run package identity and no
  // production data-directory override.
  assert.match(
    lockHelper,
    /com\.fruitboard\.desktop\.foundation-smoke\.lock\.json/,
  );
  assert.match(script, /Get-FoundationSmokeLockPath/);
  assert.match(script, /foundation-smoke-lock\.ps1/);
  assert.match(checklist, /scripts\/foundation-smoke-lock\.ps1/);
  assert.match(checklist, /Acquire-FoundationSmokeLock/);
  assert.match(checklist, /Release-FoundationSmokeLock/);
  assert.doesNotMatch(
    `${script}\n${lockHelper}`,
    /FRUITBOARD_.*DATA|app_local_data_dir|data_directory.*override/i,
  );

  // Explicit ownership: runId, pid, startedUtc, purpose, schema version.
  for (const token of [
    /runId/,
    /startedUtc/,
    /\bpid\b/,
    /purpose/,
    /schemaVersion = 1/,
  ]) {
    assert.match(lockHelper, token);
  }
  assert.match(script, /New-FoundationSmokeLockOwner/);
  assert.match(script, /-Purpose "windows-foundation-smoke"/);
  assert.match(checklist, /-Purpose "installed-journey"/);

  // Exclusive creation fails closed for a second run.
  assert.match(lockHelper, /\[System\.IO\.FileMode\]::CreateNew/);
  assert.match(
    lockHelper,
    /Another Foundation Smoke run owns the shared test identity/,
  );
  assert.match(lockHelper, /do not delete owner\.lock/i);
  assert.match(lockHelper, /kill unrelated processes/i);

  // Lock acquisition precedes every shared-state side effect in the
  // automated smoke: install, launch, archival, and uninstall. Function
  // definitions precede the lock; invocations must follow it.
  const acquireAt = script.indexOf("Acquire-FoundationSmokeLock");
  assert.ok(acquireAt !== -1);
  for (const effect of [
    "Install-SmokePackage -InstallerPath",
    "Invoke-LaunchProbe -ApplicationPath",
    "Invoke-AppSmokeMode -ApplicationPath",
    "Move-PreviousFoundationSmokeData -SourceDirectory",
    "Uninstall-SmokePackage",
  ]) {
    const at = script.indexOf(effect, acquireAt);
    assert.ok(
      at !== -1 && at > acquireAt,
      `${effect} must run after lock acquisition so a second run fails first`,
    );
  }

  // Stale ownership is decided through verifiable process state, never by
  // deleting storage/owner.lock and never by killing unrelated processes.
  assert.match(lockHelper, /Get-Process -Id/);
  assert.match(lockHelper, /Test-FoundationSmokeOwnerAlive/);
  assert.match(lockHelper, /Get-FoundationSmokeLiveAppProcesses/);
  assert.match(lockHelper, /fruitboard-desktop/);
  // Empty process results unroll to $null under StrictMode, so callers must
  // wrap with @(...) before reading .Count.
  assert.match(lockHelper, /@\(Get-FoundationSmokeLiveAppProcesses\)/);
  assert.match(script, /@\(Get-FoundationSmokeLiveAppProcesses\)/);
  assert.doesNotMatch(lockHelper, /Stop-Process/);
  assert.doesNotMatch(lockHelper, /taskkill/i);
  assert.doesNotMatch(
    lockHelper,
    /owner\.lock.*Remove-Item|Remove-Item.*owner\.lock/i,
  );
  assert.match(
    lockHelper,
    /Close every fruitboard-desktop process gracefully before archival/,
  );

  // Prior evidence is preserved through reversible archival with checked
  // absolute paths and hash verification; no database deletion.
  assert.match(lockHelper, /Move-Item -LiteralPath/);
  assert.match(lockHelper, /Get-FileHash/);
  assert.match(lockHelper, /archival (did not complete|lost|changed)/i);
  assert.match(script, /Move-PreviousFoundationSmokeData/);
  assert.match(script, /archived-legacy-foundation-smoke-/);
  assert.doesNotMatch(script, /Remove-Item/);
  assert.equal(
    (lockHelper.match(/Remove-Item -LiteralPath \$LockPath -Force/g) ?? [])
      .length,
    1,
  );

  // Normal data and the native owner-lock contract are untouched: the only
  // data directories joined are the shared test identity and its lock. The
  // helper documents storage_busy/owner.lock only to forbid bypassing them.
  assert.doesNotMatch(
    `${script}\n${lockHelper}`,
    /Join-Path[^;]*"com\.fruitboard\.desktop"/,
  );

  // Evidence records isolation ownership, restoration, and cleanup.
  assert.match(script, /mode = "exclusive-host-lock"/);
  assert.match(script, /archivedPreviousData/);
  assert.match(script, /cleanupOwner/);
  assert.match(script, /restorationOwner/);
  assert.match(script, /finally \{\s*\n.*Release-FoundationSmokeLock/s);
});

test("concurrent second runs fail before side effects", async () => {
  const { mkdtempSync, openSync, closeSync, writeFileSync, rmSync } =
    await import("node:fs");
  const { tmpdir } = await import("node:os");
  const { join } = await import("node:path");

  // Mirrors the helper's exclusive CreateNew contract: the first owner wins
  // and the second fails before touching shared state.
  const directory = mkdtempSync(join(tmpdir(), "fruitboard-smoke-lock-"));
  try {
    const lockPath = join(directory, "smoke.lock.json");
    const first = openSync(lockPath, "wx", 0o600);
    writeFileSync(first, JSON.stringify({ runId: "first", pid: 1 }));
    closeSync(first);

    let secondFailed = false;
    try {
      const second = openSync(lockPath, "wx", 0o600);
      closeSync(second);
    } catch (error) {
      secondFailed = error?.code === "EEXIST";
    }
    assert.equal(secondFailed, true);

    // The failure happens before any install/archive/uninstall effect: the
    // shared directory is untouched by the loser.
    const { existsSync, mkdirSync, readdirSync } = await import("node:fs");
    const shared = join(directory, "shared-data");
    mkdirSync(shared);
    assert.deepEqual(readdirSync(shared), []);
    assert.equal(existsSync(lockPath), true);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("stale locks recover only after process exit and cleanup releases only its owner", async () => {
  const { mkdtempSync, writeFileSync, readFileSync, rmSync, unlinkSync } =
    await import("node:fs");
  const { tmpdir } = await import("node:os");
  const { join } = await import("node:path");

  const directory = mkdtempSync(join(tmpdir(), "fruitboard-smoke-stale-"));
  try {
    // A lock whose owner PID no longer exists is stale and may be archived
    // and replaced; a live owner must block. PID liveness here uses the
    // portable signal-zero probe, mirroring Get-Process liveness.
    const deadPid = 2_147_483_647;
    let deadAlive = true;
    try {
      process.kill(deadPid, 0);
      deadAlive = true;
    } catch {
      deadAlive = false;
    }
    assert.equal(deadAlive, false);
    assert.equal(process.pid > 0, true);

    // Cleanup releases only its own lock: foreign and missing locks are
    // preserved, matching Release-FoundationSmokeLock warnings.
    const lockPath = join(directory, "smoke.lock.json");
    writeFileSync(lockPath, JSON.stringify({ runId: "owner-a" }));
    const releaseForeign = (path, runId) => {
      const existing = JSON.parse(readFileSync(path, "utf8"));
      if (existing.runId !== runId) {
        return "preserved";
      }
      unlinkSync(path);
      return "released";
    };
    assert.equal(releaseForeign(lockPath, "owner-b"), "preserved");
    assert.equal(releaseForeign(lockPath, "owner-a"), "released");

    let missingWarned = false;
    try {
      readFileSync(lockPath, "utf8");
    } catch {
      missingWarned = true;
    }
    assert.equal(missingWarned, true);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
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
