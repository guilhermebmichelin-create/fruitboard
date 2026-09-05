import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PlatformPort } from "../platform/contracts";
import { createFakePlatform } from "../platform/fake";
import { StartupRouter } from "./StartupRouter";

const containDiagnostic = () => undefined;

beforeEach(() => {
  window.history.replaceState(null, "", "/");
});

describe("StartupRouter", () => {
  it("shows an accessible loading state while reading the preference", () => {
    const platform: PlatformPort = {
      getAppHealth: () => new Promise(() => undefined),
      getStartupView: () => new Promise(() => undefined),
      setStartupView: () => new Promise(() => undefined),
    };

    render(<StartupRouter onError={containDiagnostic} platform={platform} />);

    expect(screen.getByRole("main").getAttribute("aria-busy")).toBe("true");
    expect(screen.getByRole("status").textContent).toContain(
      "Loading your startup view",
    );
  });

  it("opens a persisted startup view when no hash route was supplied", async () => {
    render(
      <StartupRouter
        onError={containDiagnostic}
        platform={createFakePlatform(undefined, "library")}
      />,
    );

    expect(
      await screen.findByRole("heading", { level: 1, name: "Library" }),
    ).toBeTruthy();
    expect(window.location.hash).toBe("#/library");
  });

  it("preserves an explicit deep link without reading the startup view", async () => {
    window.history.replaceState(null, "", "/#/board");
    const getStartupView = vi.fn().mockResolvedValue({
      startupView: "preferences",
    });
    const platform: PlatformPort = {
      getAppHealth: () =>
        Promise.resolve({
          status: "ok",
          runtime: "desktop",
          version: "0.1.0-test",
        }),
      getStartupView,
      setStartupView: vi.fn(),
    };

    render(<StartupRouter onError={containDiagnostic} platform={platform} />);

    expect(
      await screen.findByRole("heading", { level: 1, name: "Board" }),
    ).toBeTruthy();
    expect(window.location.hash).toBe("#/board");
    expect(getStartupView).not.toHaveBeenCalled();
  });

  it("contains a load failure and retries without exposing diagnostics", async () => {
    const user = userEvent.setup();
    const getStartupView = vi
      .fn()
      .mockRejectedValueOnce(
        new Error("Bearer secret C:\\Users\\producer\\private.flp"),
      )
      .mockResolvedValueOnce({ startupView: "board" });
    const platform: PlatformPort = {
      getAppHealth: () =>
        Promise.resolve({
          status: "ok",
          runtime: "desktop",
          version: "0.1.0-test",
        }),
      getStartupView,
      setStartupView: vi.fn(),
    };

    render(<StartupRouter onError={containDiagnostic} platform={platform} />);

    expect(
      await screen.findByRole("heading", {
        level: 1,
        name: "Startup preference unavailable",
      }),
    ).toBeTruthy();
    expect(document.body.textContent).not.toContain("private.flp");

    await user.click(screen.getByRole("button", { name: "Try again" }));

    expect(
      await screen.findByRole("heading", { level: 1, name: "Board" }),
    ).toBeTruthy();
    expect(getStartupView).toHaveBeenCalledTimes(2);
  });

  it("can safely continue to Home after a load failure", async () => {
    const user = userEvent.setup();
    const platform: PlatformPort = {
      getAppHealth: () =>
        Promise.resolve({
          status: "ok",
          runtime: "desktop",
          version: "0.1.0-test",
        }),
      getStartupView: () => Promise.reject(new Error("storage unavailable")),
      setStartupView: vi.fn(),
    };

    render(<StartupRouter onError={containDiagnostic} platform={platform} />);
    await screen.findByRole("alert");
    await user.click(screen.getByRole("button", { name: "Open Home" }));

    expect(
      await screen.findByRole("heading", { level: 1, name: "Home" }),
    ).toBeTruthy();
    expect(window.location.hash).toBe("#/");
  });
});
