import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { createFakePlatform } from "../platform/fake";
import { FruitboardApp } from "./FruitboardApp";

describe("FruitboardApp", () => {
  it("renders native health through a fake platform port", async () => {
    const platform = createFakePlatform({
      status: "ok",
      runtime: "desktop",
      version: "1.2.3-test",
    });

    render(<FruitboardApp platform={platform} />);

    expect(screen.getByRole("status").textContent).toContain("Checking");
    expect(await screen.findByText("Connected")).toBeTruthy();
    expect(screen.getByText("1.2.3-test")).toBeTruthy();
  });
});
