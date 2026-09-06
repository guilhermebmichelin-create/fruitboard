import { describe, expect, it, vi } from "vitest";
import {
  ADD_SCAN_ROOT_COMMAND,
  createAddScanRootArguments,
  createRemoveScanRootArguments,
  createTauriPlatform,
  LIST_SCAN_ROOTS_ARGUMENTS,
  LIST_SCAN_ROOTS_COMMAND,
  PICK_SCAN_ROOT_COMMAND,
  REMOVE_SCAN_ROOT_COMMAND,
} from "./tauri";

const correlationId = "correlation_00000000000000000000000000000001";

const scanRoot = {
  id: "root-1",
  displayName: "Projects",
  canonicalPath: "C:\\Music\\Projects",
  enabled: true,
  availability: "available",
  lastErrorCode: null,
};

describe("scan-root platform commands", () => {
  it("picks a folder through the typed native command", async () => {
    const invokeCommand = vi.fn().mockResolvedValue({
      status: "ok",
      schemaVersion: 1,
      correlationId,
      data: { selectedPath: "C:\\Music\\Projects" },
    });
    const platform = createTauriPlatform(invokeCommand);

    await expect(platform.pickScanRootDirectory()).resolves.toBe(
      "C:\\Music\\Projects",
    );
    expect(invokeCommand).toHaveBeenCalledExactlyOnceWith(
      PICK_SCAN_ROOT_COMMAND,
      { request: { schemaVersion: 1 } },
    );
  });

  it("treats picker cancellation as a null selection", async () => {
    const invokeCommand = vi.fn().mockResolvedValue({
      status: "ok",
      schemaVersion: 1,
      correlationId,
      data: { selectedPath: null },
    });
    const platform = createTauriPlatform(invokeCommand);

    await expect(platform.pickScanRootDirectory()).resolves.toBeNull();
  });

  it("rejects unusable picker responses without trusting them", async () => {
    const invokeCommand = vi.fn().mockResolvedValue({
      status: "ok",
      schemaVersion: 1,
      correlationId,
      data: { selectedPath: ["C:\\A", "C:\\B"] },
    });
    const platform = createTauriPlatform(invokeCommand);

    await expect(platform.pickScanRootDirectory()).rejects.toMatchObject({
      name: "PlatformError",
      code: "internal",
    });
  });

  it("lists, adds, and removes roots with exact typed arguments", async () => {
    const invokeCommand = vi
      .fn()
      .mockResolvedValueOnce({
        status: "ok",
        schemaVersion: 1,
        correlationId,
        data: [scanRoot],
      })
      .mockResolvedValueOnce({
        status: "ok",
        schemaVersion: 1,
        correlationId,
        data: scanRoot,
      })
      .mockResolvedValueOnce({
        status: "ok",
        schemaVersion: 1,
        correlationId,
        data: { id: "root-1" },
      });
    const platform = createTauriPlatform(invokeCommand);

    await expect(platform.listScanRoots()).resolves.toEqual([scanRoot]);
    await expect(
      platform.addScanRoot("Projects", "C:\\Music\\Projects"),
    ).resolves.toEqual(scanRoot);
    await expect(platform.removeScanRoot("root-1")).resolves.toBe("root-1");

    expect(invokeCommand).toHaveBeenNthCalledWith(
      1,
      LIST_SCAN_ROOTS_COMMAND,
      LIST_SCAN_ROOTS_ARGUMENTS,
    );
    expect(invokeCommand).toHaveBeenNthCalledWith(
      2,
      ADD_SCAN_ROOT_COMMAND,
      createAddScanRootArguments("Projects", "C:\\Music\\Projects"),
    );
    expect(invokeCommand).toHaveBeenNthCalledWith(
      3,
      REMOVE_SCAN_ROOT_COMMAND,
      createRemoveScanRootArguments("root-1"),
    );
  });

  it("validates root input before invoking native code", async () => {
    const invokeCommand = vi.fn();
    const platform = createTauriPlatform(invokeCommand);

    await expect(
      platform.addScanRoot("   ", "C:\\Music"),
    ).rejects.toMatchObject({
      code: "invalid_request",
    });
    await expect(platform.addScanRoot("Music", "")).rejects.toMatchObject({
      code: "invalid_request",
    });
    await expect(platform.removeScanRoot("")).rejects.toMatchObject({
      code: "invalid_request",
    });
    expect(invokeCommand).not.toHaveBeenCalled();
  });

  it("rejects malformed scan-root payloads without trusting them", async () => {
    const invokeCommand = vi.fn().mockResolvedValue({
      status: "ok",
      schemaVersion: 1,
      correlationId,
      data: [{ ...scanRoot, availability: "scanning" }],
    });
    const platform = createTauriPlatform(invokeCommand);

    await expect(platform.listScanRoots()).rejects.toMatchObject({
      code: "internal",
    });
  });
});
