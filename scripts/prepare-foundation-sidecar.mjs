import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

import { readToolchainPolicy } from "./lib/toolchain-policy.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const policy = readToolchainPolicy(
  join(root, "tools", "toolchain-policy.json"),
);

if (process.platform !== "win32") {
  console.error("The foundation package smoke is Windows-only.");
  process.exitCode = 1;
} else {
  const cargo = spawnSync(
    "cargo",
    [
      "build",
      "--locked",
      "--release",
      "--package",
      "fruitboard-foundation-sidecar-smoke",
      "--target",
      policy.windowsRustTarget,
    ],
    { cwd: root, stdio: "inherit" },
  );

  if (cargo.status !== 0) {
    process.exitCode = cargo.status ?? 1;
  } else {
    const source = join(
      root,
      "target",
      policy.windowsRustTarget,
      "release",
      "fruitboard-sidecar-smoke.exe",
    );
    const destination = join(
      root,
      "apps",
      "desktop",
      "src-tauri",
      "binaries",
      `fruitboard-sidecar-smoke-${policy.windowsRustTarget}.exe`,
    );
    mkdirSync(dirname(destination), { recursive: true });
    copyFileSync(source, destination);
    console.log(
      `Prepared the inert sidecar for ${policy.windowsRustTarget} from locked Rust inputs.`,
    );
  }
}
