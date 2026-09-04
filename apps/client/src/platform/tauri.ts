import { invoke } from "@tauri-apps/api/core";
import { parseAppHealth, type PlatformPort } from "./contracts";

export const GET_APP_HEALTH_COMMAND = "get_app_health";

type InvokeCommand = (command: string) => Promise<unknown>;

export function createTauriPlatform(
  invokeCommand: InvokeCommand = (command) => invoke(command),
): PlatformPort {
  return {
    async getAppHealth() {
      const health = parseAppHealth(
        await invokeCommand(GET_APP_HEALTH_COMMAND),
      );

      if (health.runtime !== "desktop") {
        throw new Error("The desktop adapter received a non-desktop response.");
      }

      return health;
    },
  };
}
