import { createRequire } from "node:module";
import { mkdir, writeFile } from "node:fs/promises";

const outputDirectory = "docs/review/phase-2-library";
const browserExecutable = process.env.CHROMIUM_EXECUTABLE;
const playwrightModulePath =
  process.env.PLAYWRIGHT_CORE_PATH ?? "playwright-core";
const { chromium } = createRequire(import.meta.url)(playwrightModulePath);

if (!browserExecutable) {
  throw new Error("CHROMIUM_EXECUTABLE is required for rendered evidence.");
}

await mkdir(outputDirectory, { recursive: true });
const browser = await chromium.launch({
  executablePath: browserExecutable,
  headless: true,
});
const evidence = [];

for (const [name, width, height] of [
  ["desktop", 1280, 1600],
  ["narrow", 390, 844],
]) {
  const context = await browser.newContext({ viewport: { width, height } });
  const page = await context.newPage();

  await page.goto("http://127.0.0.1:1420");
  await page.evaluate(async () => {
    const { createFakePlatform } = await import("/src/platform/fake.ts");
    const { createFakeLibraryScanAdapter } =
      await import("/src/library/fake.ts");
    const { mountFruitboard } = await import("/src/mount.tsx");

    const platform = createFakePlatform();
    const rootA = await platform.addScanRoot(
      "Projects",
      "C:\\Synthetic\\Music\\Projects",
    );
    const rootB = await platform.addScanRoot(
      "Projects",
      "D:\\Synthetic\\Archive\\Projects",
    );
    const records = [
      {
        locationId: "location-kick",
        rootId: rootA.id,
        rootDisplayName: rootA.displayName,
        rootCanonicalPath: rootA.canonicalPath,
        fileName: "Kick.flp",
        relativePath: "Kick.flp",
        byteSize: "49152",
        modifiedAt: "2026-08-30T13:15:00.123456789Z",
        presence: "present",
      },
      {
        locationId: "location-missing",
        rootId: rootA.id,
        rootDisplayName: rootA.displayName,
        rootCanonicalPath: rootA.canonicalPath,
        fileName: "Missing.flp",
        relativePath: "Archive\\Missing.flp",
        byteSize: "16384",
        modifiedAt: "2026-08-28T08:45:00.000Z",
        presence: "missing",
      },
      {
        locationId: "location-strings",
        rootId: rootA.id,
        rootDisplayName: rootA.displayName,
        rootCanonicalPath: rootA.canonicalPath,
        fileName: "Strings.flp",
        relativePath: "Ideas\\Strings.flp",
        byteSize: "1048576",
        modifiedAt: "2026-08-29T18:20:00.000Z",
        presence: "present",
      },
      {
        locationId: "location-vocal",
        rootId: rootB.id,
        rootDisplayName: rootB.displayName,
        rootCanonicalPath: rootB.canonicalPath,
        fileName: "Vocal.flp",
        relativePath: "Vocal.flp",
        byteSize: "8192",
        modifiedAt: "2026-08-27T11:05:00.000Z",
        presence: "present",
      },
      {
        locationId: "location-long",
        rootId: rootB.id,
        rootDisplayName: rootB.displayName,
        rootCanonicalPath: rootB.canonicalPath,
        fileName: "Long arrangement name.flp",
        relativePath: "Unreleased\\2026\\Long arrangement name.flp",
        byteSize: "32768",
        modifiedAt: "2026-08-26T09:10:00.000Z",
        presence: "present",
      },
    ];
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA, rootB],
      files: records,
      pageLimit: 4,
    });
    adapter.completeScan(rootA.id);
    adapter.completeScan(rootB.id);
    globalThis.__fruitboardFakeLibrary = adapter;

    document.querySelector("#root")?.remove();
    const container = document.createElement("div");
    container.id = "root";
    document.body.append(container);
    location.hash = "/library";
    mountFruitboard(container, platform, adapter);
  });

  await page.getByRole("heading", { name: "Your FLP library" }).waitFor();
  await page.getByText("Review harness · fake adapter").waitFor();

  const steps = [];
  const scanName = "Projects (C:\\Synthetic\\Music\\Projects)";
  const scanButton = page.getByRole("button", {
    name: `Scan now ${scanName}`,
    exact: true,
  });
  const cancelButton = page.getByRole("button", {
    name: `Cancel scan ${scanName}`,
    exact: true,
  });
  const retryButton = page.getByRole("button", {
    name: `Retry scan ${scanName}`,
    exact: true,
  });

  async function state(step) {
    const value = await page.evaluate(() => ({
      focus:
        document.activeElement?.getAttribute("aria-label") ||
        document.activeElement?.textContent?.trim() ||
        document.activeElement?.tagName,
      pageOverflow:
        document.documentElement.scrollWidth > innerWidth ||
        document.body.scrollWidth > innerWidth,
      viewport: [innerWidth, innerHeight],
    }));
    if (value.pageOverflow) {
      throw new Error(`${name}: page has horizontal overflow`);
    }
    if (
      await page
        .getByText("Progress is shown as counters; no percentage is estimated.")
        .count()
    ) {
      const bodyText = await page.locator("body").textContent();
      if (bodyText?.match(/\d+%/)) {
        throw new Error(`${name}: guessed percentage rendered`);
      }
    }
    steps.push({ step, ...value });
  }

  async function assertFocus(locator, label) {
    await locator.waitFor();
    const accessibleLabel = await locator.getAttribute("aria-label");
    if (!accessibleLabel) {
      throw new Error(
        `${name}: ${label} control is missing its accessible name`,
      );
    }
    await page.waitForFunction((expectedLabel) => {
      const control = Array.from(document.querySelectorAll("button")).find(
        (element) => element.getAttribute("aria-label") === expectedLabel,
      );
      return (
        control instanceof HTMLButtonElement &&
        !control.disabled &&
        control === document.activeElement
      );
    }, accessibleLabel);
  }

  await page.screenshot({
    path: `${outputDirectory}/${name}-populated.png`,
    fullPage: true,
  });
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement)
      document.activeElement.blur();
  });
  for (let index = 0; index < 80; index += 1) {
    await page.keyboard.press("Tab");
    if (
      await scanButton.evaluate((element) => element === document.activeElement)
    )
      break;
  }
  await assertFocus(scanButton, "Scan now");
  await state("Tab reaches Scan now");

  await page.keyboard.press("Enter");
  await page.getByText("Queued", { exact: true }).waitFor();
  await assertFocus(cancelButton, "Queued focus moves to Cancel");
  await state("Enter queues scan");
  await page.screenshot({
    path: `${outputDirectory}/${name}-queued.png`,
    fullPage: true,
  });

  await page.evaluate(() => {
    globalThis.__fruitboardFakeLibrary.advanceRun("root-1");
  });
  await page.getByText("Running", { exact: true }).waitFor();
  await assertFocus(cancelButton, "Running keeps Cancel focus");
  await state("Fake adapter advances to running");

  await page.keyboard.press("Enter");
  await page.getByText("Cancelled", { exact: true }).waitFor();
  await page
    .getByRole("heading", { name: "Showing previous committed results" })
    .waitFor();
  await assertFocus(retryButton, "Cancelled focus moves to Retry");
  await state("Cancel retains previous committed results");
  await page.screenshot({
    path: `${outputDirectory}/${name}-cancelled-stale.png`,
    fullPage: true,
  });

  await page.keyboard.press("Enter");
  await page.getByText("Queued", { exact: true }).waitFor();
  await assertFocus(cancelButton, "Retry focus moves to Cancel");
  await state("Enter retries scan");
  await page.screenshot({
    path: `${outputDirectory}/${name}-retried.png`,
    fullPage: true,
  });

  evidence.push({
    adapter: "stateful fake; no native enumeration, permissions, or SQLite",
    name,
    width,
    height,
    steps,
  });
  await context.close();
}

await browser.close();
await writeFile(
  `${outputDirectory}/keyboard-trace.json`,
  `${JSON.stringify(evidence, null, 2)}\n`,
);
console.log(
  "PASS desktop/narrow Library focus, stale retention, keyboard, and overflow assertions",
);
