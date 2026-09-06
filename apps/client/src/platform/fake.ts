import {
  PlatformError,
  type AppHealth,
  type NativeErrorCode,
  type PlatformPort,
  type ScanRoot,
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
  let scanRoots: ScanRoot[] = [];
  let sequence = 0;

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
    pickScanRootDirectory() {
      return Promise.resolve(null);
    },
    listScanRoots() {
      return Promise.resolve([...scanRoots]);
    },
    addScanRoot(displayName, path) {
      sequence += 1;
      const root: ScanRoot = {
        id: `root-${sequence}`,
        displayName,
        canonicalPath: path,
        enabled: true,
        availability: "available",
        lastErrorCode: null,
      };
      scanRoots = [...scanRoots, root];
      return Promise.resolve(root);
    },
    removeScanRoot(id) {
      scanRoots = scanRoots.filter((root) => root.id !== id);
      return Promise.resolve(id);
    },
  };
}

export function createFakePlatformWithPickedDirectory(
  picked: string | null,
): PlatformPort {
  return {
    ...createFakePlatform(),
    pickScanRootDirectory() {
      return Promise.resolve(picked);
    },
  };
}

type ScanRootMethods = Pick<
  PlatformPort,
  "pickScanRootDirectory" | "listScanRoots" | "addScanRoot" | "removeScanRoot"
>;

// Shared stubs for test platforms that only exercise other slices. The
// never-settling variants keep loading states stable; the rejecting variants
// reuse the owning test's error so diagnostics stay realistic.
export const pendingScanRootMethods: ScanRootMethods = {
  pickScanRootDirectory: () => new Promise<string | null>(() => undefined),
  listScanRoots: () => new Promise<readonly ScanRoot[]>(() => undefined),
  addScanRoot: () => new Promise<ScanRoot>(() => undefined),
  removeScanRoot: () => new Promise<string>(() => undefined),
};

export const failingScanRootMethods = (error: unknown): ScanRootMethods => ({
  pickScanRootDirectory: () => Promise.reject(error),
  listScanRoots: () => Promise.reject(error),
  addScanRoot: () => Promise.reject(error),
  removeScanRoot: () => Promise.reject(error),
});

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
    pickScanRootDirectory() {
      return Promise.reject(new PlatformError(code));
    },
    listScanRoots() {
      return Promise.reject(new PlatformError(code));
    },
    addScanRoot() {
      return Promise.reject(new PlatformError(code));
    },
    removeScanRoot() {
      return Promise.reject(new PlatformError(code));
    },
  };
}
