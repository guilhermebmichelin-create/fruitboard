import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { readToolchainPolicy } from "../scripts/lib/toolchain-policy.mjs";

const policy = readToolchainPolicy();
const readRootFile = (path) =>
  readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

const readHexToken = (css, name) => {
  const value = css.match(new RegExp(`--${name}:\\s*(#[0-9a-f]{6})`, "i"))?.[1];
  assert.ok(value, `missing hexadecimal token --${name}`);
  return value;
};

const relativeLuminance = (hex) => {
  const channels = hex
    .slice(1)
    .match(/.{2}/g)
    .map((channel) => Number.parseInt(channel, 16) / 255)
    .map((channel) =>
      channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4,
    );

  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
};

const contrastRatio = (first, second) => {
  const luminances = [relativeLuminance(first), relativeLuminance(second)].sort(
    (left, right) => right - left,
  );
  return (luminances[0] + 0.05) / (luminances[1] + 0.05);
};

test("JavaScript manifests agree with the central toolchain policy", () => {
  const packageJson = JSON.parse(readRootFile("package.json"));
  assert.equal(packageJson.engines.node, policy.node);
  assert.equal(packageJson.engines.pnpm, policy.pnpm);
  assert.match(
    packageJson.packageManager,
    new RegExp(`^pnpm@${policy.pnpm}\\+sha512\\.`),
  );
  assert.equal(packageJson.devDependencies.corepack, policy.corepack);
});

test("Rust and Python pins agree with the central policy", () => {
  assert.equal(readRootFile(".node-version").trim(), policy.node);
  assert.equal(readRootFile(".python-version").trim(), policy.python);
  assert.match(
    readRootFile("rust-toolchain.toml"),
    new RegExp(`channel = "${policy.rust}"`),
  );
  assert.match(
    readRootFile("pyproject.toml"),
    /requires-python = "==3\.11\.\*"/,
  );
});

test("research environment contains no PyFLP dependency", () => {
  const dependencyFiles = `${readRootFile("pyproject.toml")}\n${readRootFile("uv.lock")}`;
  assert.doesNotMatch(dependencyFiles, /pyflp/i);
});

test("privacy and generated-output ignore rules remain present", () => {
  const gitignore = readRootFile(".gitignore");
  for (const pattern of [
    "*.flp",
    "*.fst",
    "*.wav",
    "*.mp3",
    "*.flac",
    ".env",
    "node_modules/",
    "target/",
    ".venv/",
  ]) {
    assert.match(
      gitignore,
      new RegExp(`^${pattern.replaceAll("*", "\\*")}$`, "m"),
    );
  }
});

test("application manifests use exact dependency versions", () => {
  for (const path of [
    "apps/client/package.json",
    "apps/desktop/package.json",
    "packages/ui/package.json",
  ]) {
    const manifest = JSON.parse(readRootFile(path));

    for (const section of ["dependencies", "devDependencies"]) {
      for (const [name, version] of Object.entries(manifest[section] ?? {})) {
        if (name.startsWith("@fruitboard/")) {
          assert.equal(
            version,
            "workspace:*",
            `${path} must resolve ${name} from this workspace`,
          );
          continue;
        }

        assert.match(
          version,
          /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/,
          `${path} must pin ${name} exactly`,
        );
      }
    }
  }
});

test("the shared UI package owns the accessible shell token vocabulary", () => {
  const clientManifest = JSON.parse(readRootFile("apps/client/package.json"));
  const uiManifest = JSON.parse(readRootFile("packages/ui/package.json"));
  const tokens = readRootFile("packages/ui/src/tokens.css");
  const clientStyles = readRootFile("apps/client/src/styles.css");

  assert.equal(clientManifest.dependencies["@fruitboard/ui"], "workspace:*");
  assert.equal(uiManifest.exports["./tokens.css"], "./src/tokens.css");
  assert.match(clientStyles, /@import "@fruitboard\/ui\/tokens\.css"/);
  assert.doesNotMatch(clientStyles, /#[0-9a-f]{3,8}\b|\brgb\(/i);

  for (const tokenGroup of [
    "font-family",
    "font-size",
    "space",
    "color-surface",
    "color-border",
    "border-width",
    "radius",
    "shadow",
    "duration",
    "focus-outline",
    "icon-size",
    "control-size",
  ]) {
    assert.match(tokens, new RegExp(`--${tokenGroup}`));
  }

  assert.match(clientStyles, /prefers-reduced-motion: reduce/);
  assert.match(clientStyles, /forced-colors: active/);
  assert.match(clientStyles, /var\(--control-size-touch\)/);
  assert.match(clientStyles, /var\(--duration-spin\)/);
  assert.doesNotMatch(clientStyles, /0\.0625rem solid/);

  for (const [foreground, background] of [
    ["color-text", "color-surface"],
    ["color-text-muted", "color-surface"],
    ["color-text-on-dark-muted", "color-sidebar"],
    ["color-accent-strong", "color-accent-soft"],
    ["color-text-on-dark", "color-accent"],
    ["color-danger", "color-danger-soft"],
  ]) {
    assert.ok(
      contrastRatio(
        readHexToken(tokens, foreground),
        readHexToken(tokens, background),
      ) >= 4.5,
      `${foreground} must retain WCAG AA text contrast on ${background}`,
    );
  }

  assert.ok(
    contrastRatio(
      readHexToken(tokens, "color-focus"),
      readHexToken(tokens, "color-surface"),
    ) >= 3,
    "the focus indicator must retain 3:1 non-text contrast",
  );
});

test("the Phase 1 shell does not introduce PWA or scanner authority", () => {
  const clientManifest = JSON.parse(readRootFile("apps/client/package.json"));
  const dependencyNames = Object.keys({
    ...clientManifest.dependencies,
    ...clientManifest.devDependencies,
  }).join("\n");
  const navigation = readRootFile("apps/client/src/app/navigation.ts");

  assert.doesNotMatch(dependencyNames, /(?:workbox|vite-plugin-pwa)/i);
  assert.doesNotMatch(navigation, /(?:scanner|service worker)/i);
});

test("Cargo workspace contains only the first owned native package", () => {
  const cargoManifest = readRootFile("Cargo.toml");

  assert.match(cargoManifest, /members = \["apps\/desktop\/src-tauri"\]/);
  assert.doesNotMatch(
    cargoManifest,
    /(?:scanner|storage-sqlite|parser-protocol)/,
  );
});
