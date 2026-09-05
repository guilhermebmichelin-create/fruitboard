import {
  PlatformError,
  type AppHealth,
  type NativeErrorCode,
  type PlatformPort,
  type StartupView,
} from "./contracts";

const defaultHealth: AppHealth = {
  status: "ok",
  runtime: "desktop",
  version: "0.1.0-test",
};

export function createFakePlatform(
  health: AppHealth = defaultHealth,
  initialStartupView: StartupView = "home",
): PlatformPort {
  let startupView = initialStartupView;

  return {
    getAppHealth() {
      return Promise.resolve(health);
    },
    getStartupView() {
      return Promise.resolve({ startupView });
    },
    setStartupView(nextStartupView) {
      startupView = nextStartupView;
      return Promise.resolve({ startupView });
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
    getStartupView() {
      return Promise.reject(new PlatformError(code));
    },
    setStartupView() {
      return Promise.reject(new PlatformError(code));
    },
  };
}
