import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter } from "react-router";
import { RouterProvider } from "react-router/dom";
import { describe, expect, it, vi } from "vitest";
import type { PlatformPort } from "../platform/contracts";
import {
  createFailingPlatform,
  createFakePlatform,
  pendingScanRootMethods,
} from "../platform/fake";
import { createFruitboardMemoryRouter, createRoutes } from "./router";

const readyPlatform = createFakePlatform({
  status: "ok",
  runtime: "desktop",
  version: "1.2.3-test",
});

const loadingPlatform: PlatformPort = {
  getAppHealth: () => new Promise(() => undefined),
  getStartupView: () => new Promise(() => undefined),
  setStartupView: () => new Promise(() => undefined),
  ...pendingScanRootMethods,
};

const errorPlatform = createFailingPlatform();

function BrokenPage(): never {
  throw new Error("Bearer secret C:\\Users\\producer\\private.flp");
}

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

  it("renders correct metadata for case-insensitive and trailing-slash routes", async () => {
    for (const entry of ["/library/", "/LIBRARY"]) {
      const view = renderApp(entry);

      expect(
        await screen.findByRole("heading", { level: 1, name: "Library" }),
      ).toBeTruthy();
      expect(document.title).toBe("Library · Fruitboard");
      expect(screen.queryByText("Page not found")).toBeNull();
      expect(screen.getByRole("link", { name: "Library" }).ariaCurrent).toBe(
        "page",
      );
      view.unmount();
    }
  });

  it("contains route render failures without displaying diagnostics", async () => {
    const routes = createRoutes(readyPlatform);
    const rootRoute = routes[0];
    if (!rootRoute?.children) {
      throw new Error("test route tree is missing its root children");
    }
    rootRoute.children.push({ path: "broken", element: <BrokenPage /> });
    const router = createMemoryRouter(routes, { initialEntries: ["/broken"] });
    const onError = vi.fn();
    const containUntrustedRendererDiagnostic = () => undefined;
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);

    try {
      render(<RouterProvider onError={onError} router={router} />, {
        onCaughtError: containUntrustedRendererDiagnostic,
      });

      expect(
        await screen.findByRole("heading", {
          level: 1,
          name: "Fruitboard needs to restart",
        }),
      ).toBeTruthy();
      expect(screen.getByRole("alert")).toBeTruthy();
      expect(document.body.textContent).not.toContain("secret");
      expect(document.body.textContent).not.toContain("private.flp");
      await waitFor(() => {
        expect(document.title).toBe("Application error · Fruitboard");
      });
      expect(onError).toHaveBeenCalledOnce();
      expect(consoleError).not.toHaveBeenCalled();
    } finally {
      consoleError.mockRestore();
    }
  });
});
