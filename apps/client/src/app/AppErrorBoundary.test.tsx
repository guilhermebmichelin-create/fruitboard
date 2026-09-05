import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AppErrorBoundary } from "./AppErrorBoundary";

function BrokenView(): never {
  throw new Error("Bearer secret C:\\Users\\producer\\private.flp");
}

describe("AppErrorBoundary", () => {
  it("contains render failures without displaying diagnostics", () => {
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);

    try {
      render(
        <AppErrorBoundary>
          <BrokenView />
        </AppErrorBoundary>,
      );
    } finally {
      consoleError.mockRestore();
    }

    expect(screen.getByRole("alert")).toHaveProperty(
      "textContent",
      expect.stringContaining("Fruitboard needs to restart"),
    );
    expect(document.body.textContent).not.toContain("secret");
    expect(document.body.textContent).not.toContain("private.flp");
  });
});
