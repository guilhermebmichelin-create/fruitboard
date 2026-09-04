export type PlatformRuntime = "desktop" | "web";

export interface AppHealth {
  readonly status: "ok";
  readonly runtime: PlatformRuntime;
  readonly version: string;
}

export interface PlatformPort {
  getAppHealth(): Promise<AppHealth>;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

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
