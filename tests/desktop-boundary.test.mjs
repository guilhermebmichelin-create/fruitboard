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

// The library seam's hand-maintained catalog: every code, message, and
// retryable classification must match `errors.rs` or the client silently
// degrades native errors to `internal`.
const libraryErrors = [
  {
    code: "access_denied",
    rustVariant: "AccessDenied",
    message: "The requested location could not be accessed.",
  },
  {
    code: "unavailable",
    rustVariant: "Unavailable",
    message: "The requested service is temporarily unavailable.",
  },
  {
    code: "unsupported",
    rustVariant: "Unsupported",
    message: "The requested operation is not supported for this location.",
  },
  {
    code: "resource_limit",
    rustVariant: "ResourceLimit",
    message: "The operation exceeded a resource limit.",
  },
  {
    code: "conflict",
    rustVariant: "Conflict",
    message: "The request could not be completed because its state changed.",
  },
  {
    code: "not_found",
    rustVariant: "NotFound",
    message: "The requested item is no longer available.",
  },
  {
    code: "cancelled",
    rustVariant: "Cancelled",
    message: "The operation was cancelled.",
  },
  {
    code: "internal",
    rustVariant: "Internal",
    message: "Fruitboard could not complete the request.",
  },
  {
    code: "invalid_cursor",
    rustVariant: "InvalidCursor",
    message: "The page continuation is not valid; start from the first page.",
  },
  {
    code: "stale_cursor",
    rustVariant: "StaleCursor",
    message: "The list changed; start again from the first page.",
  },
];

test("desktop capability exposes only health, preference, scan-root, and scan-console commands", () => {
  const capability = JSON.parse(
    readRootFile("apps/desktop/src-tauri/capabilities/main.json"),
  );
  const permission = readRootFile(
    "apps/desktop/src-tauri/permissions/health.toml",
  );
  const preferencesPermission = readRootFile(
    "apps/desktop/src-tauri/permissions/preferences.toml",
  );
  const scanRootsPermission = readRootFile(
    "apps/desktop/src-tauri/permissions/scan-roots.toml",
  );
  const scanConsolePermission = readRootFile(
    "apps/desktop/src-tauri/permissions/scan-console.toml",
  );

  assert.equal(capability.local, true);
  assert.equal(capability.remote, undefined);
  assert.deepEqual(capability.windows, ["main"]);
  assert.deepEqual(capability.permissions, [
    "allow-get-app-health",
    "allow-get-startup-view",
    "allow-set-startup-view",
    "allow-list-scan-roots",
    "allow-add-scan-root",
    "allow-remove-scan-root",
    "allow-pick-scan-root",
    "allow-set-scan-root-display-name",
    "allow-set-scan-root-enabled",
    "allow-scan-now",
    "allow-cancel-scan",
    "allow-retry-scan",
    "allow-list-scan-statuses",
    "allow-get-library-page",
    "allow-get-project-details",
    "allow-request-project-analysis",
    "allow-get-scan-console-state",
    "core:event:allow-listen",
    "core:event:allow-unlisten",
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
  for (const command of [
    "list_scan_roots",
    "add_scan_root",
    "remove_scan_root",
    "pick_scan_root",
    "set_scan_root_display_name",
    "set_scan_root_enabled",
  ]) {
    assert.match(
      scanRootsPermission,
      new RegExp(`commands\\.allow = \\["${command}"\\]`),
    );
  }
  for (const command of [
    "scan_now",
    "cancel_scan",
    "retry_scan",
    "list_scan_statuses",
    "get_library_page",
    "get_project_details",
    "request_project_analysis",
    "get_scan_console_state",
  ]) {
    assert.match(
      scanConsolePermission,
      new RegExp(`commands\\.allow = \\["${command}"\\]`),
    );
  }
  assert.doesNotMatch(
    `${JSON.stringify(capability)}\n${permission}\n${preferencesPermission}\n${scanRootsPermission}\n${scanConsolePermission}`,
    /(?:dialog|fs|shell|sql|process|opener):/,
  );
  assert.doesNotMatch(
    JSON.stringify(capability.permissions),
    /core:event:allow-(?:emit|emit-to)/,
  );
});

test("native scan subscriptions have scoped listen and unlisten capabilities", () => {
  const capability = JSON.parse(
    readRootFile("apps/desktop/src-tauri/capabilities/main.json"),
  );
  const tauriAdapter = readRootFile("apps/client/src/platform/tauri.ts");
  const nativeLibraryAdapter = readRootFile(
    "apps/client/src/library/native.ts",
  );

  assert.deepEqual(
    capability.permissions.filter((permission) =>
      permission.startsWith("core:event:"),
    ),
    ["core:event:allow-listen", "core:event:allow-unlisten"],
  );
  assert.match(
    tauriAdapter,
    /import \{ listen \} from "@tauri-apps\/api\/event";/,
  );
  assert.match(tauriAdapter, /listen\(event, \(\) => handler\(\)\)/);
  assert.match(
    nativeLibraryAdapter,
    /transport\.listen\(SCAN_STATUS_CHANGED_EVENT/,
  );
  assert.match(nativeLibraryAdapter, /if \(unlisten !== null\) unlisten\(\)/);
  assert.doesNotMatch(
    JSON.stringify(capability.permissions),
    /(?:core:event:allow-(?:emit|emit-to)|(?:^|:)(?:shell|fs):)/,
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
    "apps/client/src/app/ScanRootsManager.tsx",
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
    "list_scan_roots",
    "add_scan_root",
    "remove_scan_root",
    "pick_scan_root",
    "set_scan_root_display_name",
    "set_scan_root_enabled",
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

test("library error catalog stays aligned with the Rust vocabulary", () => {
  const rustErrors = readRootFile(
    "apps/desktop/src-tauri/src/foundation/errors.rs",
  );
  const nativeLibrary = readRootFile("apps/client/src/library/native.ts");

  for (const { code, rustVariant, message } of libraryErrors) {
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
      nativeLibrary,
      new RegExp(`${code}:\\s*"${escapeRegExp(message)}"`),
      `TypeScript library mapping for ${code}`,
    );
  }

  // Retryable classification must match `ErrorCode::retryable`.
  assert.match(
    rustErrors,
    /matches!\(self, Self::Conflict \| Self::Unavailable\)/,
  );
  assert.match(
    nativeLibrary,
    /const LIBRARY_RETRYABLE = new Set<LibraryErrorCode>\(\[\s*"conflict",\s*"unavailable",?\s*\]\)/,
  );

  // The runtime code gate must cover exactly the documented catalog.
  const codesBlock =
    nativeLibrary.match(
      /const LIBRARY_ERROR_CODES = new Set<string>\(\[([\s\S]*?)\]\);/,
    )?.[1] ?? "";
  const declaredCodes = [...codesBlock.matchAll(/"([a-z_]+)"/g)].map(
    ([, declared]) => declared,
  );
  assert.deepEqual(
    [...declaredCodes].sort(),
    libraryErrors.map(({ code }) => code).sort(),
  );

  // Completeness in the other direction: every Rust user-facing message is
  // pinned by one of the two TypeScript catalogs, so adding or rewording a
  // Rust code cannot silently surface as `internal` to the client.
  const rustImpl = rustErrors.slice(
    rustErrors.indexOf("impl ErrorCode"),
    rustErrors.indexOf("pub(crate) enum DiagnosticCode"),
  );
  const rustMappings = [...rustImpl.matchAll(/Self::(\w+) => "([^"]+)"/g)].map(
    ([, variant, mapped]) => [variant, mapped],
  );
  assert.equal(
    rustMappings.length,
    11,
    "the Rust user-facing vocabulary is exactly the 11 documented codes",
  );
  const pinned = new Map();
  for (const { rustVariant, message } of [...nativeErrors, ...libraryErrors]) {
    pinned.set(rustVariant, message);
  }
  for (const [variant, mapped] of rustMappings) {
    assert.equal(
      pinned.get(variant),
      mapped,
      `both TypeScript catalogs must pin the Rust mapping for ${variant}`,
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
