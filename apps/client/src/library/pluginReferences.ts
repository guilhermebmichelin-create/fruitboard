import { LibraryAdapterError } from "./contracts";

export type PluginReason =
  | "PLUGIN_NAME_NOT_STORED"
  | "PLUGIN_VENDOR_NOT_STORED"
  | "PLUGIN_NAME_ENCODING_UNSUPPORTED"
  | "VST_METADATA_UNSUPPORTED"
  | "MULTIPLE_VST_METADATA_RECORDS";
export type ProjectPluginDetail =
  | { readonly status: "extracted"; readonly value: string }
  | {
      readonly status: "inferred";
      readonly value: "Sampler";
      readonly method: "sampler-default-for-known-build";
      readonly confidence: "high";
    }
  | {
      readonly status: "unavailable";
      readonly value: null;
      readonly reason: "PLUGIN_NAME_NOT_STORED" | "PLUGIN_VENDOR_NOT_STORED";
    }
  | {
      readonly status: "unsupported";
      readonly value: null;
      readonly reason:
        | "PLUGIN_NAME_ENCODING_UNSUPPORTED"
        | "VST_METADATA_UNSUPPORTED"
        | "MULTIPLE_VST_METADATA_RECORDS";
    };
export interface ProjectPluginReference {
  /** One-based reference order, not a channel number or stable plugin ID. */
  readonly position: number;
  readonly className: ProjectPluginDetail;
  readonly name: ProjectPluginDetail;
  readonly vendor: ProjectPluginDetail;
}
export interface ProjectPluginReferences {
  readonly coverage: "top-level-saved-references";
  readonly items: readonly ProjectPluginReference[];
}
const record = (v: unknown): v is Record<string, unknown> =>
  typeof v === "object" && v !== null && !Array.isArray(v);
const fail = (): never => {
  throw new LibraryAdapterError("internal");
};
function field(
  value: unknown,
  role: "class" | "name" | "vendor",
  build: string,
): ProjectPluginDetail {
  if (
    !record(value) ||
    value["items"] !== undefined ||
    value["assumptions"] !== undefined
  )
    return fail();
  const status = value["status"];
  const text = value["value"];
  if (status === "extracted") {
    if (
      typeof text !== "string" ||
      text.length === 0 ||
      text.length > 4095 ||
      text.includes("\0") ||
      value["reason"] !== undefined ||
      value["method"] !== undefined ||
      value["confidence"] !== undefined
    )
      return fail();
    return { status, value: text };
  }
  if (status === "inferred") {
    if (
      role !== "name" ||
      !["25.1.3.4922", "26.1.0.5530"].includes(build) ||
      text !== "Sampler" ||
      value["method"] !== "sampler-default-for-known-build" ||
      value["confidence"] !== "high" ||
      value["reason"] !== undefined
    )
      return fail();
    return {
      status,
      value: "Sampler",
      method: "sampler-default-for-known-build",
      confidence: "high",
    };
  }
  if (
    text !== null ||
    value["method"] !== undefined ||
    value["confidence"] !== undefined
  )
    return fail();
  const reason = value["reason"];
  if (status === "unavailable") {
    const expected =
      role === "vendor" ? "PLUGIN_VENDOR_NOT_STORED" : "PLUGIN_NAME_NOT_STORED";
    if (reason !== expected) return fail();
    return { status, value: null, reason: expected };
  }
  if (
    status !== "unsupported" ||
    (reason !== "PLUGIN_NAME_ENCODING_UNSUPPORTED" &&
      reason !== "VST_METADATA_UNSUPPORTED" &&
      reason !== "MULTIPLE_VST_METADATA_RECORDS")
  )
    return fail();
  return { status, value: null, reason };
}
function consistent(item: ProjectPluginReference): boolean {
  const { className: klass, name, vendor } = item;
  const vendorAbsent =
    vendor.status === "unavailable" &&
    vendor.reason === "PLUGIN_VENDOR_NOT_STORED";
  if (klass.status === "extracted") {
    if (klass.value !== "Fruity Wrapper")
      return (
        name.status === "extracted" &&
        name.value === klass.value &&
        vendorAbsent
      );
    if (
      (name.status === "extracted" || name.status === "unavailable") &&
      (vendor.status === "extracted" || vendor.status === "unavailable")
    )
      return true;
    if (
      name.status === "unsupported" &&
      name.reason === "VST_METADATA_UNSUPPORTED" &&
      vendorAbsent
    )
      return true;
    return (
      name.status === "unsupported" &&
      vendor.status === "unsupported" &&
      name.reason === vendor.reason &&
      (name.reason === "VST_METADATA_UNSUPPORTED" ||
        name.reason === "MULTIPLE_VST_METADATA_RECORDS")
    );
  }
  if (klass.status === "unavailable")
    return name.status === "inferred" && vendorAbsent;
  return (
    klass.status === "unsupported" &&
    klass.reason === "PLUGIN_NAME_ENCODING_UNSUPPORTED" &&
    name.status === "unsupported" &&
    name.reason === "PLUGIN_NAME_ENCODING_UNSUPPORTED" &&
    vendorAbsent
  );
}
export function parseProjectPlugins(
  value: unknown,
  build: string,
): ProjectPluginReferences {
  if (
    !record(value) ||
    value["coverage"] !== "top-level-saved-references" ||
    !Array.isArray(value["items"]) ||
    value["items"].length > 1024 ||
    ["value", "status", "reason", "method", "confidence"].some(
      (key) => value[key] !== undefined,
    )
  )
    return fail();
  const items = value["items"].map(
    (entry: unknown, index): ProjectPluginReference => {
      if (!record(entry) || entry["position"] !== index + 1) return fail();
      const item = {
        position: index + 1,
        className: field(entry["className"], "class", build),
        name: field(entry["name"], "name", build),
        vendor: field(entry["vendor"], "vendor", build),
      };
      if (!consistent(item)) return fail();
      return item;
    },
  );
  const result: ProjectPluginReferences = {
    coverage: "top-level-saved-references",
    items,
  };
  if (new TextEncoder().encode(JSON.stringify(result)).length > 256 * 1024)
    return fail();
  return result;
}
export const PLUGIN_REASON_COPY: Readonly<Record<PluginReason, string>> = {
  PLUGIN_NAME_NOT_STORED: "No name was stored.",
  PLUGIN_VENDOR_NOT_STORED: "No vendor was stored.",
  PLUGIN_NAME_ENCODING_UNSUPPORTED: "The saved name encoding is unsupported.",
  VST_METADATA_UNSUPPORTED:
    "Saved wrapper name/vendor information is unsupported or was not reported.",
  MULTIPLE_VST_METADATA_RECORDS:
    "Multiple saved wrapper records prevent a reliable name/vendor display.",
};
