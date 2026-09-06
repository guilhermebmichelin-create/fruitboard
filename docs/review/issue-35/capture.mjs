import { chromium } from "playwright-core";
import { mkdir, writeFile } from "node:fs/promises";
const out = "docs/review/issue-35";
await mkdir(out, { recursive: true });
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_EXECUTABLE,
  headless: true,
});
const evidence = [];
for (const [name, width, height] of [
  ["desktop", 1280, 800],
  ["narrow", 390, 844],
]) {
  const context = await browser.newContext({ viewport: { width, height } });
  const page = await context.newPage();
  await page.goto("http://127.0.0.1:1420");
  await page.evaluate(async () => {
    const { createFakePlatform } = await import("/src/platform/fake.ts");
    const { mountFruitboard } = await import("/src/mount.tsx");
    const platform = createFakePlatform();
    await platform.addScanRoot("Projects", "C:\\Synthetic\\Music\\Projects");
    await platform.addScanRoot("Projects", "D:\\Synthetic\\Archive\\Projects");
    document.querySelector("#root").remove();
    const el = document.createElement("div");
    el.id = "root";
    document.body.append(el);
    location.hash = "/preferences";
    mountFruitboard(el, platform);
  });
  const rename = page.getByRole("button", {
    name: "Rename Projects (C:\\Synthetic\\Music\\Projects)",
    exact: true,
  });
  await rename.waitFor();
  const steps = [];
  async function assertFocus(locator, label) {
    await locator.waitFor();
    await page.waitForTimeout(50);
    if (!(await locator.evaluate((el) => el === document.activeElement))) {
      throw Error(`${label}: focus is not on the expected control`);
    }
  }

  async function state(step) {
    const value = await page.evaluate(() => ({
      focus:
        document.activeElement?.getAttribute("aria-label") ||
        document.activeElement?.textContent ||
        document.activeElement?.tagName,
      overflow: document.documentElement.scrollWidth > innerWidth,
      viewport: [innerWidth, innerHeight],
    }));
    if (value.overflow) throw Error(name + " horizontal overflow");
    steps.push({ step, ...value });
  }
  // Reach the control with real Tab events from the document, without programmatic focus.
  for (let i = 0; i < 40; i++) {
    await page.keyboard.press("Tab");
    if (await rename.evaluate((el) => el === document.activeElement)) break;
  }
  if (!(await rename.evaluate((el) => el === document.activeElement)))
    throw Error("Rename not keyboard reachable");
  await state("Tab to first same-name root Rename");
  await page.screenshot({
    path: `${out}/${name}-keyboard.png`,
    fullPage: true,
  });
  await page.keyboard.press("Enter");
  if (
    !(await page
      .getByLabel("Folder name", { exact: true })
      .evaluate((el) => el === document.activeElement))
  )
    throw Error("Editor focus");
  await page.keyboard.press("Control+A");
  await page.keyboard.type("Draft cancelled");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  if (
    !(await page
      .getByRole("button", { name: "Cancel", exact: true })
      .evaluate((el) => el === document.activeElement))
  )
    throw Error("Cancel focus");
  await state("Tab to explicit Cancel");
  await page.screenshot({ path: `${out}/${name}-cancel.png`, fullPage: true });
  await page.keyboard.press("Enter");
  await page.waitForTimeout(100);
  if (!(await rename.evaluate((el) => el === document.activeElement)))
    throw Error("Cancel did not restore focus");
  await state("Cancel restores Rename");
  await page.keyboard.press("Enter");
  await page.keyboard.press("Escape");
  await page.waitForTimeout(100);
  if (!(await rename.evaluate((el) => el === document.activeElement)))
    throw Error("Escape focus");
  await state("Escape restores Rename");
  await page.keyboard.press("Enter");
  await page.keyboard.press("Control+A");
  await page.keyboard.type("Released");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  const saved = page.getByRole("button", {
    name: "Rename Released",
    exact: true,
  });
  await saved.waitFor();
  await page.waitForTimeout(100);
  if (!(await saved.evaluate((el) => el === document.activeElement)))
    throw Error("Save focus");
  await state("Save restores renamed control");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  const keepReleased = page.getByRole("button", {
    name: "Keep Released",
    exact: true,
  });
  await assertFocus(keepReleased, "Remove opens confirmation");
  await state("Remove opens confirmation with Keep focused");
  await page.screenshot({
    path: `${out}/${name}-confirmation.png`,
    fullPage: true,
  });
  await page.keyboard.press("Enter");
  const removeReleased = page.getByRole("button", {
    name: "Remove Released",
    exact: true,
  });
  await assertFocus(removeReleased, "Keep cancels removal");
  await state("Keep cancels and restores Remove focus");

  await page.keyboard.press("Enter");
  await assertFocus(keepReleased, "Second confirmation");
  await state("Second confirmation keeps focus safe");
  await page.keyboard.press("Shift+Tab");
  const confirmReleased = page.getByRole("button", {
    name: "Confirm removal of Released",
    exact: true,
  });
  await assertFocus(confirmReleased, "Confirm removal is keyboard reachable");
  await page.keyboard.press("Enter");
  const addFolder = page.getByRole("button", {
    name: "Add folder",
    exact: true,
  });
  await page.getByText("Folder removed.").waitFor();
  await assertFocus(addFolder, "Successful removal restores Add folder focus");
  await state("Removal restores Add folder focus");
  evidence.push({
    adapter: "stateful fake; no native picker or SQLite",
    name,
    width,
    height,
    steps,
  });
  await context.close();
}
await browser.close();
await writeFile(
  `${out}/keyboard-trace.json`,
  JSON.stringify(evidence, null, 2) + "\n",
);
console.log(
  "PASS desktop/narrow keyboard, Cancel/Escape/save focus and no horizontal overflow",
);
