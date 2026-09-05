import { describe, expect, it, vi } from "vitest";
import {
  createTauriPlatform,
  GET_APP_HEALTH_ARGUMENTS,
  GET_APP_HEALTH_COMMAND,
} from "./tauri";

const correlationId = "correlation_00000000000000000000000000000001";

describe("createTauriPlatform", () => {
  it("invokes only the typed health command", async () => {
    const invokeCommand = vi.fn().mockResolvedValue({
      status: "ok",
      schemaVersion: 1,
      correlationId,
      data: {
        status: "ok",
        runtime: "desktop",
        version: "0.1.0",
      },
    });
    const platform = createTauriPlatform(invokeCommand);

    await expect(platform.getAppHealth()).resolves.toEqual({
      status: "ok",
      runtime: "desktop",
      version: "0.1.0",
    });
    expect(invokeCommand).toHaveBeenCalledExactlyOnceWith(
      GET_APP_HEALTH_COMMAND,
      GET_APP_HEALTH_ARGUMENTS,
    );
  });

  it("preserves stable user-safe native errors", async () => {
    const platform = createTauriPlatform(
      vi.fn().mockResolvedValue({
        status: "error",
        schemaVersion: 1,
        correlationId,
        error: {
          code: "invalid_request",
          message: "The request was not valid.",
          retryable: false,
        },
      }),
    );

    await expect(platform.getAppHealth()).rejects.toMatchObject({
      code: "invalid_request",
      correlationId,
      message: "The request was not valid.",
      retryable: false,
    });
  });

  it("maps malformed and unknown native errors to an internal error", async () => {
    for (const response of [
      {
        status: "ok",
        correlationId,
        data: { status: "ok", runtime: "desktop", version: "0.1.0" },
      },
      {
        status: "ok",
        schemaVersion: 2,
        correlationId,
        data: { status: "ok", runtime: "desktop", version: "0.1.0" },
      },
      {
        status: "ok",
        schemaVersion: 1,
        correlationId,
        data: { status: "ok" },
      },
      {
        status: "error",
        schemaVersion: 1,
        correlationId,
        error: {
          code: "future_error",
          message: "C:\\Users\\producer\\private.flp",
          retryable: true,
        },
      },
    ]) {
      const platform = createTauriPlatform(vi.fn().mockResolvedValue(response));

      await expect(platform.getAppHealth()).rejects.toMatchObject({
        code: "internal",
        message: "Fruitboard could not complete the request.",
      });
    }
  });

  it("rejects a non-desktop runtime", async () => {
    const platform = createTauriPlatform(
      vi.fn().mockResolvedValue({
        status: "ok",
        schemaVersion: 1,
        correlationId,
        data: {
          status: "ok",
          runtime: "web",
          version: "0.1.0",
        },
      }),
    );

    await expect(platform.getAppHealth()).rejects.toMatchObject({
      code: "internal",
      message: "Fruitboard could not complete the request.",
    });
  });

  it("does not expose rejected invoke diagnostics", async () => {
    const platform = createTauriPlatform(
      vi
        .fn()
        .mockRejectedValue(
          new Error("Bearer secret C:\\Users\\producer\\private.flp"),
        ),
    );

    await expect(platform.getAppHealth()).rejects.toMatchObject({
      code: "unavailable",
      correlationId: null,
      message: "The requested service is temporarily unavailable.",
    });
  });
});
