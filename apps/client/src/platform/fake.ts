import {
  PlatformError,
  type AppHealth,
  type NativeErrorCode,
  type PlatformPort,
} from "./contracts";

const defaultHealth: AppHealth = {
  status: "ok",
  runtime: "desktop",
  version: "0.1.0-test",
};

export function createFakePlatform(
  health: AppHealth = defaultHealth,
): PlatformPort {
  return {
    getAppHealth() {
      return Promise.resolve(health);
    },
  };
}

export function createFailingPlatform(
  code: NativeErrorCode = "unavailable",
): PlatformPort {
  return {
    getAppHealth() {
      return Promise.reject(new PlatformError(code));
    },
  };
}
