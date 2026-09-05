import { invoke } from "@tauri-apps/api/core";
import {
  NATIVE_COMMAND_SCHEMA_VERSION,
  parseAppHealth,
  PlatformError,
  unwrapCommandEnvelope,
  type PlatformPort,
} from "./contracts";

export const GET_APP_HEALTH_COMMAND = "get_app_health";
export const GET_APP_HEALTH_ARGUMENTS = Object.freeze({
  request: Object.freeze({ schemaVersion: NATIVE_COMMAND_SCHEMA_VERSION }),
});

type InvokeCommand = (
  command: string,
  arguments_: Record<string, unknown>,
) => Promise<unknown>;

export function createTauriPlatform(
  invokeCommand: InvokeCommand = (command, arguments_) =>
    invoke(command, arguments_),
): PlatformPort {
  return {
    async getAppHealth() {
      try {
        return unwrapCommandEnvelope(
          await invokeCommand(GET_APP_HEALTH_COMMAND, GET_APP_HEALTH_ARGUMENTS),
          (data) => {
            const health = parseAppHealth(data);
            if (health.runtime !== "desktop") {
              throw new Error("unexpected runtime");
            }
            return health;
          },
        );
      } catch (error) {
        if (error instanceof PlatformError) {
          throw error;
        }
        throw new PlatformError("unavailable");
      }
    },
  };
}
