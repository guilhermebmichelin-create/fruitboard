import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const readRootFile = (path) =>
  readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

test("desktop capability exposes only the health command to the local window", () => {
  const capability = JSON.parse(
    readRootFile("apps/desktop/src-tauri/capabilities/main.json"),
  );
  const permission = readRootFile(
    "apps/desktop/src-tauri/permissions/health.toml",
  );

  assert.equal(capability.local, true);
  assert.equal(capability.remote, undefined);
  assert.deepEqual(capability.windows, ["main"]);
  assert.deepEqual(capability.permissions, ["allow-get-app-health"]);
  assert.match(permission, /commands\.allow = \["get_app_health"\]/);
  assert.doesNotMatch(
    `${JSON.stringify(capability)}\n${permission}`,
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
    "apps/client/src/app/FruitboardApp.tsx",
    "apps/client/src/mount.tsx",
    "apps/client/src/platform/contracts.ts",
    "apps/client/src/platform/fake.ts",
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
