import { invoke } from "@tauri-apps/api/core";
import {
  NATIVE_COMMAND_SCHEMA_VERSION,
  parseAppHealth,
  parseStartupViewPreference,
  PlatformError,
  unwrapCommandEnvelope,
  type PlatformPort,
  type StartupView,
} from "./contracts";

export const GET_APP_HEALTH_COMMAND = "get_app_health";
export const GET_APP_HEALTH_ARGUMENTS = Object.freeze({
  request: Object.freeze({ schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION }),
});
export const GET_STARTUP_VIEW_COMMAND = "get_startup_view";
export const GET_STARTUP_VIEW_ARGUMENTS = Object.freeze({
  request: Object.freeze({ schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION }),
});
export const SET_STARTUP_VIEW_COMMAND = "set_startup_view";

export const createSetStartupViewArguments = (startupView: StartupView) =>
  Object.freeze({
    request: Object.freeze({
      schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION,
      startupView,
    }),
  });

type InvokeCommand = (
  command: string,
  arguments_: Record<string, unknown>,
) => Promise<unknown>;

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
  };
}
