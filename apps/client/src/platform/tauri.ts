import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  NATIVE_COMMAND_SCHEMA_VERSION,
  parseAppHealth,
  parseScanRoot,
  parseScanRootList,
  parseStartupViewPreference,
  PlatformError,
  unwrapCommandEnvelope,
  type PlatformPort,
  type StartupView,
} from "./contracts";
import {
  createNativeLibraryScanAdapter,
  SCAN_STATUS_CHANGED_EVENT,
  type NativeLibraryTransport,
} from "../library/native";
import type { LibraryScanAdapter } from "../library/contracts";

export const GET_APP_HEALTH_COMMAND = "get_app_health";
export const GET_APP_HEALTH_ARGUMENTS = Object.freeze({
  request: Object.freeze({ schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION }),
});
export const GET_STARTUP_VIEW_COMMAND = "get_startup_view";
export const GET_STARTUP_VIEW_ARGUMENTS = Object.freeze({
  request: Object.freeze({ schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION }),
});
export const SET_STARTUP_VIEW_COMMAND = "set_startup_view";
export const PICK_SCAN_ROOT_COMMAND = "pick_scan_root";
export const LIST_SCAN_ROOTS_COMMAND = "list_scan_roots";
export const ADD_SCAN_ROOT_COMMAND = "add_scan_root";
export const REMOVE_SCAN_ROOT_COMMAND = "remove_scan_root";
export const SET_SCAN_ROOT_DISPLAY_NAME_COMMAND = "set_scan_root_display_name";
export const SET_SCAN_ROOT_ENABLED_COMMAND = "set_scan_root_enabled";

const PICK_SCAN_ROOT_ARGUMENTS = Object.freeze({
  request: Object.freeze({ schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION }),
});
export const LIST_SCAN_ROOTS_ARGUMENTS = Object.freeze({
  request: Object.freeze({ schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION }),
});

export const createAddScanRootArguments = (displayName: string, path: string) =>
  Object.freeze({
    request: Object.freeze({
      schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION,
      displayName,
      path,
    }),
  });

export const createRemoveScanRootArguments = (id: string) =>
  Object.freeze({
    request: Object.freeze({
      schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION,
      id,
    }),
  });

export const createSetScanRootDisplayNameArguments = (
  id: string,
  displayName: string,
) =>
  Object.freeze({
    request: Object.freeze({
      schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION,
      id,
      displayName,
    }),
  });

export const createSetScanRootEnabledArguments = (
  id: string,
  enabled: boolean,
) =>
  Object.freeze({
    request: Object.freeze({
      schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION,
      id,
      enabled,
    }),
  });

export const createSetStartupViewArguments = (startupView: StartupView) =>
  Object.freeze({
    request: Object.freeze({
      schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION,
      startupView,
    }),
  });

export const SCAN_CONSOLE_COMMANDS = Object.freeze([
  "scan_now",
  "cancel_scan",
  "retry_scan",
  "list_scan_statuses",
  "get_library_page",
  "get_scan_console_state",
] as const);

export type ScanConsoleCommand = (typeof SCAN_CONSOLE_COMMANDS)[number];

export function createTauriLibraryTransport(
  invokeCommand: NativeLibraryTransport["invoke"] = (command, arguments_) =>
    invoke(command, arguments_),
  listenEvent: NativeLibraryTransport["listen"] = (event, handler) =>
    listen(event, () => handler()),
): NativeLibraryTransport {
  return {
    invoke: invokeCommand,
    listen: listenEvent,
  };
}

/**
 * Native `LibraryScanAdapter` behind the scan-console IPC surface.
 *
 * Production wiring for `entries/desktop.tsx`: the six scan-console
 * commands are permitted in `capabilities/main.json` (all other commands
 * keep `removeUnusedCommands=true`). With the `scan-console` cargo feature
 * off every command returns the typed `unavailable` envelope, which the
 * adapter surfaces as a recoverable `LibraryAdapterError("unavailable")`
 * for the existing Library refresh/error UI. Subscription uses the typed
 * `scan-status-changed` event; unsubscription is safe on route changes and
 * unmount (see `LibraryPage` mount/adapter-lifetime cleanup).
 */
export function createTauriLibraryScanAdapter(
  transport: NativeLibraryTransport = createTauriLibraryTransport(),
): LibraryScanAdapter {
  return createNativeLibraryScanAdapter(transport);
}

export { SCAN_STATUS_CHANGED_EVENT };

type InvokeCommand = (
  command: string,
  arguments_: Record<string, unknown>,
) => Promise<unknown>;

function parsePickedDirectory(value: unknown): string | null {
  if (value === null) {
    return null;
  }
  if (typeof value === "string" && value.length > 0) {
    return value;
  }
  throw new Error("The folder picker returned an unusable selection.");
}

function parseRemovedScanRoot(id: string, value: unknown): string {
  if (!(
    typeof value === "object" &&
    value !== null &&
    (value as Record<string, unknown>)["id"] === id
  )) {
    throw new Error("The platform returned an unexpected removal response.");
  }
  return id;
}

export function createTauriPlatform(
  invokeCommand: InvokeCommand = (command, arguments_) =>
    invoke(command, arguments_),
): PlatformPort {
  const execute = async <Result>(
    command: string,
    arguments_: Record<string, unknown>,
    parseData: (data: unknown) => Result,
  ): Promise<Result> => {
    try {
      return unwrapCommandEnvelope(
        await invokeCommand(command, arguments_),
        parseData,
      );
    } catch (error) {
      if (error instanceof PlatformError) {
        throw error;
      }
      throw new PlatformError("unavailable");
    }
  };

  return {
    async getAppHealth() {
      return execute(
        GET_APP_HEALTH_COMMAND,
        GET_APP_HEALTH_ARGUMENTS,
        (data) => {
          const health = parseAppHealth(data);
          if (health.runtime !== "desktop") {
            throw new Error("unexpected runtime");
          }
          return health;
        },
      );
    },
    async getStartupView() {
      return execute(
        GET_STARTUP_VIEW_COMMAND,
        GET_STARTUP_VIEW_ARGUMENTS,
        parseStartupViewPreference,
      );
    },
    async setStartupView(startupView) {
      try {
        parseStartupViewPreference({ startupView });
      } catch {
        throw new PlatformError("invalid_request");
      }
      const preference = await execute(
        SET_STARTUP_VIEW_COMMAND,
        createSetStartupViewArguments(startupView),
        parseStartupViewPreference,
      );
      if (preference.startupView !== startupView) {
        throw new PlatformError("internal");
      }
      return preference;
    },
    async pickScanRootDirectory() {
      const selection = await execute(
        PICK_SCAN_ROOT_COMMAND,
        PICK_SCAN_ROOT_ARGUMENTS,
        (data) => {
          if (
            typeof data !== "object" ||
            data === null ||
            !("selectedPath" in data)
          ) {
            throw new Error(
              "The platform returned an invalid picker response.",
            );
          }
          return parsePickedDirectory(
            (data as Record<string, unknown>)["selectedPath"],
          );
        },
      );
      return selection;
    },
    async listScanRoots() {
      return execute(
        LIST_SCAN_ROOTS_COMMAND,
        LIST_SCAN_ROOTS_ARGUMENTS,
        parseScanRootList,
      );
    },
    async addScanRoot(displayName, path) {
      if (displayName.trim() === "" || path === "") {
        throw new PlatformError("invalid_request");
      }
      return execute(
        ADD_SCAN_ROOT_COMMAND,
        createAddScanRootArguments(displayName, path),
        parseScanRoot,
      );
    },
    async removeScanRoot(id) {
      if (id === "") {
        throw new PlatformError("invalid_request");
      }
      return execute(
        REMOVE_SCAN_ROOT_COMMAND,
        createRemoveScanRootArguments(id),
        (data) => parseRemovedScanRoot(id, data),
      );
    },
    async updateScanRootDisplayName(id, displayName) {
      if (id === "" || displayName.trim() === "") {
        throw new PlatformError("invalid_request");
      }
      const root = await execute(
        SET_SCAN_ROOT_DISPLAY_NAME_COMMAND,
        createSetScanRootDisplayNameArguments(id, displayName),
        parseScanRoot,
      );
      if (root.id !== id || root.displayName !== displayName.trim()) {
        throw new PlatformError("internal");
      }
      return root;
    },
    async setScanRootEnabled(id, enabled) {
      if (id === "") {
        throw new PlatformError("invalid_request");
      }
      const root = await execute(
        SET_SCAN_ROOT_ENABLED_COMMAND,
        createSetScanRootEnabledArguments(id, enabled),
        parseScanRoot,
      );
      if (root.id !== id || root.enabled !== enabled) {
        throw new PlatformError("internal");
      }
      return root;
    },
  };
}
