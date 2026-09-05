import { render, screen } from "@testing-library/react";
import axe from "axe-core";
import { RouterProvider } from "react-router/dom";
import { describe, expect, it } from "vitest";
import type { PlatformPort } from "../platform/contracts";
import { createFakePlatform } from "../platform/fake";
import { createFruitboardMemoryRouter } from "./router";
import { StartupRouter } from "./StartupRouter";

const readyPlatform = createFakePlatform();

const loadingPlatform: PlatformPort = {
  getAppHealth: () => new Promise(() => undefined),
  getStartupView: () => new Promise(() => undefined),
  setStartupView: () => new Promise(() => undefined),
};

const errorPlatform: PlatformPort = {
  getAppHealth: () => Promise.reject(new Error("native host unavailable")),
  getStartupView: () => Promise.reject(new Error("native host unavailable")),
  setStartupView: () => Promise.reject(new Error("native host unavailable")),
};

async function expectAccessible(
  initialEntry: string,
  platform: PlatformPort,
  settledText?: string,
) {
  const router = createFruitboardMemoryRouter(platform, [initialEntry]);
  const view = render(<RouterProvider router={router} />);

  if (settledText) {
    await screen.findByText(settledText);
  }

  await expectNoAutomatedViolations(view.container);
  view.unmount();
}

async function expectNoAutomatedViolations(container: HTMLElement) {
  // axe cannot calculate rendered color contrast in jsdom. The workspace
  // policy test compensates by enforcing the shell's 4.5:1 text and 3:1 focus
  // token pairs.
  const results = await axe.run(container, {
    rules: {
      "color-contrast": { enabled: false },
    },
  });

  expect(
    results.violations.map(({ help, id, nodes }) => ({
      help,
      id,
      targets: nodes.map((node) => node.target),
    })),
  ).toEqual([]);
}

describe("application shell accessibility", () => {
  it("has no automated violations in the initial state", async () => {
    await expectAccessible("/", readyPlatform, "Connected");
  });

  it("has no automated violations in the loading state", async () => {
    await expectAccessible("/", loadingPlatform);
  });

  it("has no automated violations in the empty state", async () => {
    await expectAccessible("/library", readyPlatform, "Connected");
  });

  it("has no automated violations in route and connection errors", async () => {
    await expectAccessible("/missing", readyPlatform, "Connected");
    await expectAccessible("/", errorPlatform, "Connection unavailable");
  });

  it("has no automated violations in the startup preference state", async () => {
    await expectAccessible(
      "/preferences",
      readyPlatform,
      "Home is the default startup view.",
    );
  });

  it("has no automated violations in startup loading and error states", async () => {
    window.history.replaceState(null, "", "/");
    const loadingView = render(
      <StartupRouter onError={() => undefined} platform={loadingPlatform} />,
    );
    await expectNoAutomatedViolations(loadingView.container);
    loadingView.unmount();

    window.history.replaceState(null, "", "/");
    const errorView = render(
      <StartupRouter onError={() => undefined} platform={errorPlatform} />,
    );
    await screen.findByText("Startup preference unavailable");
    await expectNoAutomatedViolations(errorView.container);
  });
});
