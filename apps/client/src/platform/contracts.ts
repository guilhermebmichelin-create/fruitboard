export type PlatformRuntime = "desktop" | "web";

export const NATIVE_COMMAND_SCHEMA_VERSION = 1;
export const STARTUP_VIEWS = [
  "home",
  "library",
  "board",
  "preferences",
] as const;

export type StartupView = (typeof STARTUP_VIEWS)[number];

export type NativeErrorCode =
  | "invalid_request"
  | "cancelled"
  | "not_found"
  | "conflict"
  | "unavailable"
  | "internal";

export interface AppHealth {
  readonly status: "ok";
  readonly runtime: PlatformRuntime;
  readonly version: string;
}

export interface StartupViewPreference {
  readonly startupView: StartupView;
}

export interface PlatformPort {
  getAppHealth(): Promise<AppHealth>;
  getStartupView(): Promise<StartupViewPreference>;
  setStartupView(startupView: StartupView): Promise<StartupViewPreference>;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

const errorMessages: Readonly<Record<NativeErrorCode, string>> = {
  invalid_request: "The request was not valid.",
  cancelled: "The operation was cancelled.",
  not_found: "The requested item is no longer available.",
  conflict: "The request could not be completed because its state changed.",
  unavailable: "The requested service is temporarily unavailable.",
  internal: "Fruitboard could not complete the request.",
};

const retryableCodes = new Set<NativeErrorCode>(["conflict", "unavailable"]);
const nativeErrorCodes = new Set<NativeErrorCode>([
  "invalid_request",
  "cancelled",
  "not_found",
  "conflict",
  "unavailable",
  "internal",
]);
const startupViews = new Set<StartupView>(STARTUP_VIEWS);

const isNativeErrorCode = (value: unknown): value is NativeErrorCode =>
  typeof value === "string" && nativeErrorCodes.has(value as NativeErrorCode);

const isOpaqueId = (
  value: unknown,
  prefix: "correlation" | "job",
): value is string =>
  typeof value === "string" &&
  new RegExp(`^${prefix}_[0-9a-f]{32}$`).test(value);

export class PlatformError extends Error {
  readonly code: NativeErrorCode;
  readonly correlationId: string | null;
  readonly retryable: boolean;

  constructor(code: NativeErrorCode, correlationId: string | null = null) {
    super(errorMessages[code]);
    this.name = "PlatformError";
    this.code = code;
    this.correlationId = correlationId;
    this.retryable = retryableCodes.has(code);
  }
}

export function unwrapCommandEnvelope<T>(
  value: unknown,
  parseData: (data: unknown) => T,
): T {
  if (
    !isRecord(value) ||
    value["schemaVersion"] !== NATIVE_COMMAND_SCHEMA_VERSION ||
    !isOpaqueId(value["correlationId"], "correlation")
  ) {
    throw new PlatformError("internal");
  }

  const correlationId = value["correlationId"];

  if (value["status"] === "error") {
    const error = value["error"];
    if (!isRecord(error) || !isNativeErrorCode(error["code"])) {
      throw new PlatformError("internal", correlationId);
    }

    const code = error["code"];
    if (
      error["message"] !== errorMessages[code] ||
      error["retryable"] !== retryableCodes.has(code)
    ) {
      throw new PlatformError("internal", correlationId);
    }

    throw new PlatformError(code, correlationId);
  }

  if (value["status"] !== "ok" || !("data" in value)) {
    throw new PlatformError("internal", correlationId);
  }

  try {
    return parseData(value["data"]);
  } catch {
    throw new PlatformError("internal", correlationId);
  }
}

export function parseAppHealth(value: unknown): AppHealth {
  if (
    !isRecord(value) ||
    value["status"] !== "ok" ||
    (value["runtime"] !== "desktop" && value["runtime"] !== "web") ||
    typeof value["version"] !== "string" ||
    value["version"].length === 0
  ) {
    throw new Error("The platform returned an invalid health response.");
  }

  return {
    status: value["status"],
    runtime: value["runtime"],
    version: value["version"],
  };
}

export function parseStartupViewPreference(
  value: unknown,
): StartupViewPreference {
  if (
    !isRecord(value) ||
    Object.keys(value).length !== 1 ||
    typeof value["startupView"] !== "string" ||
    !startupViews.has(value["startupView"] as StartupView)
  ) {
    throw new Error("The platform returned an invalid startup preference.");
  }

  return { startupView: value["startupView"] as StartupView };
}
