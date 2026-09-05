import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const readRootFile = (path) =>
  readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

const escapeRegExp = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

const nativeErrors = [
  {
    code: "invalid_request",
    rustVariant: "InvalidRequest",
    message: "The request was not valid.",
  },
  {
    code: "cancelled",
    rustVariant: "Cancelled",
    message: "The operation was cancelled.",
  },
  {
    code: "not_found",
    rustVariant: "NotFound",
    message: "The requested item is no longer available.",
  },
  {
    code: "conflict",
    rustVariant: "Conflict",
    message: "The request could not be completed because its state changed.",
  },
  {
    code: "unavailable",
    rustVariant: "Unavailable",
    message: "The requested service is temporarily unavailable.",
  },
  {
    code: "internal",
    rustVariant: "Internal",
    message: "Fruitboard could not complete the request.",
  },
];

const startupViews = [
  ["home", "Home"],
  ["library", "Library"],
  ["board", "Board"],
  ["preferences", "Preferences"],
];

test("desktop capability exposes only health and startup-view commands", () => {
  const capability = JSON.parse(
    readRootFile("apps/desktop/src-tauri/capabilities/main.json"),
  );
  const permission = readRootFile(
    "apps/desktop/src-tauri/permissions/health.toml",
  );
  const preferencesPermission = readRootFile(
    "apps/desktop/src-tauri/permissions/preferences.toml",
  );

  assert.equal(capability.local, true);
  assert.equal(capability.remote, undefined);
  assert.deepEqual(capability.windows, ["main"]);
  assert.deepEqual(capability.permissions, [
    "allow-get-app-health",
    "allow-get-startup-view",
    "allow-set-startup-view",
  ]);
  assert.match(permission, /commands\.allow = \["get_app_health"\]/);
  assert.match(
    preferencesPermission,
    /commands\.allow = \["get_startup_view"\]/,
  );
  assert.match(
    preferencesPermission,
    /commands\.allow = \["set_startup_view"\]/,
  );
  assert.doesNotMatch(
    `${JSON.stringify(capability)}\n${permission}\n${preferencesPermission}`,
    /(?:fs|shell|sql|process|opener):/,
  );
});

test("desktop shell loads only local build and development content", () => {
  const config = JSON.parse(
    readRootFile("apps/desktop/src-tauri/tauri.conf.json"),
  );
  const developmentUrl = new URL(config.build.devUrl);

  assert.equal(developmentUrl.hostname, "127.0.0.1");
  assert.equal(config.build.frontendDist, "../../client/dist");
  assert.equal(config.build.removeUnusedCommands, true);
  assert.equal(config.app.withGlobalTauri, false);
  assert.deepEqual(config.app.security.capabilities, ["main"]);
  assert.equal(config.app.security.csp["base-uri"], "'none'");
  assert.equal(config.app.security.csp["frame-ancestors"], "'none'");
  assert.equal(config.app.security.csp["object-src"], "'none'");
  assert.doesNotMatch(
    JSON.stringify(config.app.security.csp).replace("http://ipc.localhost", ""),
    /https?:\/\//,
  );
  assert.equal(config.bundle.active, false);
  assert.deepEqual(config.bundle.icon, ["icons/icon.ico"]);
});

test("shared client modules do not import Tauri APIs", () => {
  const sharedFiles = [
    "apps/client/src/app/AppIcon.tsx",
    "apps/client/src/app/AppErrorBoundary.tsx",
    "apps/client/src/app/FruitboardApp.tsx",
    "apps/client/src/app/navigation.ts",
    "apps/client/src/app/pages.tsx",
    "apps/client/src/app/router.tsx",
    "apps/client/src/app/StartupRouter.tsx",
    "apps/client/src/app/StartupViewPreference.tsx",
    "apps/client/src/mount.tsx",
    "apps/client/src/platform/contracts.ts",
    "apps/client/src/platform/fake.ts",
    "packages/ui/src/tokens.css",
  ];

  for (const path of sharedFiles) {
    assert.doesNotMatch(readRootFile(path), /@tauri-apps\//, path);
  }
});

test("desktop version sources stay aligned", () => {
  const clientManifest = JSON.parse(readRootFile("apps/client/package.json"));
  const desktopManifest = JSON.parse(readRootFile("apps/desktop/package.json"));
  const tauriConfig = JSON.parse(
    readRootFile("apps/desktop/src-tauri/tauri.conf.json"),
  );
  const cargoManifest = readRootFile("apps/desktop/src-tauri/Cargo.toml");
  const cargoVersion = cargoManifest.match(/^version = "([^"]+)"$/m)?.[1];

  assert.equal(clientManifest.version, desktopManifest.version);
  assert.equal(desktopManifest.version, tauriConfig.version);
  assert.equal(tauriConfig.version, cargoVersion);
});

test("native command contract stays aligned across Rust and TypeScript", () => {
  const rustCommand = readRootFile(
    "apps/desktop/src-tauri/src/foundation/command.rs",
  );
  const rustErrors = readRootFile(
    "apps/desktop/src-tauri/src/foundation/errors.rs",
  );
  const clientContracts = readRootFile("apps/client/src/platform/contracts.ts");
  const tauriAdapter = readRootFile("apps/client/src/platform/tauri.ts");
  const nativeHost = readRootFile("apps/desktop/src-tauri/src/lib.rs");
  const storage = readRootFile("crates/storage-sqlite/src/lib.rs");
  const initialMigration = readRootFile(
    "crates/storage-sqlite/migrations/001_local_settings.sql",
  );

  assert.match(rustCommand, /pub const COMMAND_SCHEMA_VERSION: u64 = 1;/);
  assert.match(
    clientContracts,
    /export const NATIVE_COMMAND_SCHEMA_VERSION = 1;/,
  );
  assert.match(tauriAdapter, /schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION/);
  assert.match(
    rustErrors,
    /matches!\(self, Self::Conflict \| Self::Unavailable\)/,
  );
  assert.match(
    clientContracts,
    /new Set<NativeErrorCode>\(\["conflict", "unavailable"\]\)/,
  );
  assert.match(
    nativeHost,
    /serde\(deny_unknown_fields, rename_all = "camelCase"\)/,
  );

  for (const command of [
    "get_app_health",
    "get_startup_view",
    "set_startup_view",
  ]) {
    assert.match(nativeHost, new RegExp(`commands\\.execute\\("${command}"`));
    assert.match(tauriAdapter, new RegExp(`"${command}"`));
  }

  for (const [value, rustVariant] of startupViews) {
    assert.match(
      storage,
      new RegExp(`Self::${rustVariant}\\s*=>\\s*"${value}"`),
    );
    assert.match(clientContracts, new RegExp(`"${value}"`));
    assert.match(initialMigration, new RegExp(`'${value}'`));
  }

  for (const { code, rustVariant, message } of nativeErrors) {
    assert.equal(
      rustVariant.replace(/([a-z])([A-Z])/g, "$1_$2").toLowerCase(),
      code,
    );
    assert.match(
      rustErrors,
      new RegExp(`Self::${rustVariant}\\s*=>\\s*"${escapeRegExp(message)}"`),
      `Rust mapping for ${code}`,
    );
    assert.match(
      clientContracts,
      new RegExp(`${code}:\\s*"${escapeRegExp(message)}"`),
      `TypeScript mapping for ${code}`,
    );
  }
});

test("renderer error hooks do not print untrusted Error objects", () => {
  const mount = readRootFile("apps/client/src/mount.tsx");

  for (const callback of [
    "onCaughtError",
    "onRecoverableError",
    "onUncaughtError",
  ]) {
    assert.match(
      mount,
      new RegExp(`${callback}: containUntrustedRendererDiagnostic`),
    );
  }
  assert.match(mount, /onError=\{containUntrustedRendererDiagnostic\}/);
  assert.doesNotMatch(mount, /console\.(?:debug|error|info|log|warn)/);
});
