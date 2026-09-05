import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RouterProvider } from "react-router/dom";
import { describe, expect, it } from "vitest";
import type { PlatformPort } from "../platform/contracts";
import { createFailingPlatform, createFakePlatform } from "../platform/fake";
import { createFruitboardMemoryRouter } from "./router";

const readyPlatform = createFakePlatform({
  status: "ok",
  runtime: "desktop",
  version: "1.2.3-test",
});

const loadingPlatform: PlatformPort = {
  getAppHealth: () => new Promise(() => undefined),
};

const errorPlatform = createFailingPlatform();

function renderApp(initialEntry = "/", platform: PlatformPort = readyPlatform) {
  const router = createFruitboardMemoryRouter(platform, [initialEntry]);

  return {
    router,
    ...render(<RouterProvider router={router} />),
  };
}

describe("FruitboardApp", () => {
  it("renders the routed shell and native health through a fake platform", async () => {
    renderApp();

    expect(screen.getByRole("banner")).toBeTruthy();
    expect(screen.getByRole("navigation", { name: "Primary" })).toBeTruthy();
    expect(screen.getByRole("main")).toBeTruthy();
    expect(
      screen.getByRole("heading", { level: 1, name: "Home" }),
    ).toBeTruthy();
    expect(document.querySelector('[data-shell-state="initial"]')).toBeTruthy();

    expect(await screen.findByText("Connected")).toBeTruthy();
    expect(screen.getByText("v1.2.3-test")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Home" }).ariaCurrent).toBe("page");
  });

  it("keeps skip navigation first and moves focus to routed content", async () => {
    const user = userEvent.setup();
    renderApp();
    await screen.findByText("Connected");

    await user.tab();
    expect(document.activeElement).toBe(
      screen.getByRole("link", { name: "Skip to main content" }),
    );

    await user.tab();
    expect(document.activeElement).toBe(
      screen.getByRole("link", { name: "Home" }),
    );

    await user.tab();
    const libraryLink = screen.getByRole("link", { name: "Library" });
    expect(document.activeElement).toBe(libraryLink);

    await user.keyboard("{Enter}");
    expect(
      await screen.findByRole("heading", { level: 1, name: "Library" }),
    ).toBeTruthy();
    await waitFor(() => {
      expect(document.activeElement).toBe(screen.getByRole("main"));
    });
  });

  it("lets the skip link focus the main content without changing routes", async () => {
    const user = userEvent.setup();
    const { router } = renderApp();

    await user.click(
      screen.getByRole("link", { name: "Skip to main content" }),
    );

    expect(document.activeElement).toBe(screen.getByRole("main"));
    expect(router.state.location.pathname).toBe("/");
  });

  it("represents loading, empty, and navigation-error states", () => {
    const loadingView = renderApp("/", loadingPlatform);
    expect(
      screen.getByRole("status", { name: "Desktop connection status" }),
    ).toHaveProperty("textContent", expect.stringContaining("Connecting"));
    loadingView.unmount();

    const emptyView = renderApp("/library");
    expect(screen.getByText("No projects yet")).toBeTruthy();
    expect(
      emptyView.container.querySelector('[data-shell-state="empty"]'),
    ).toBeTruthy();
    emptyView.unmount();

    const errorView = renderApp("/missing");
    expect(screen.getByText("That page is not available")).toBeTruthy();
    expect(
      errorView.container.querySelector('[data-shell-state="error"]'),
    ).toBeTruthy();
  });

  it("contains native failures in the global error region", async () => {
    renderApp("/", errorPlatform);

    expect(
      await screen.findByRole("alert", { name: "Desktop connection error" }),
    ).toHaveProperty(
      "textContent",
      expect.stringContaining("Connection unavailable"),
    );
  });
});
