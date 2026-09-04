import { describe, expect, it, vi } from "vitest";
import { createTauriPlatform, GET_APP_HEALTH_COMMAND } from "./tauri";

describe("createTauriPlatform", () => {
  it("invokes only the typed health command", async () => {
    const invokeCommand = vi.fn().mockResolvedValue({
      status: "ok",
      runtime: "desktop",
      version: "0.1.0",
    });
    const platform = createTauriPlatform(invokeCommand);

    await expect(platform.getAppHealth()).resolves.toEqual({
      status: "ok",
      runtime: "desktop",
      version: "0.1.0",
    });
    expect(invokeCommand).toHaveBeenCalledExactlyOnceWith(
      GET_APP_HEALTH_COMMAND,
    );
  });

  it("rejects a malformed native response", async () => {
    const platform = createTauriPlatform(
      vi.fn().mockResolvedValue({ status: "ok", version: "0.1.0" }),
    );

    await expect(platform.getAppHealth()).rejects.toThrow(
      "invalid health response",
    );
  });

  it("rejects a non-desktop runtime", async () => {
    const platform = createTauriPlatform(
      vi.fn().mockResolvedValue({
        status: "ok",
        runtime: "web",
        version: "0.1.0",
      }),
    );

    await expect(platform.getAppHealth()).rejects.toThrow(
      "non-desktop response",
    );
  });
});
