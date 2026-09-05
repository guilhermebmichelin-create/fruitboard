import { spawnSync } from "node:child_process";
import { join } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

if (process.platform !== "win32") {
  console.error("The foundation packaging smoke is Windows-only.");
  process.exitCode = 1;
} else {
  const environment = Object.fromEntries(
    Object.entries(process.env).filter(
      ([name]) => name.toLowerCase() !== "psmodulepath",
    ),
  );
  const root = fileURLToPath(new URL("../", import.meta.url));
  const result = spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      join(root, "scripts", "windows-foundation-smoke.ps1"),
      ...process.argv.slice(2),
    ],
    {
      cwd: root,
      env: environment,
      stdio: "inherit",
    },
  );

  if (result.error) {
    console.error("The Windows PowerShell smoke launcher failed to start.");
    process.exitCode = 1;
  } else {
    process.exitCode = result.status ?? 1;
  }
}
