import type {
  ProjectPluginReference,
  ProjectPluginReferences,
} from "./pluginReferences";

/** Synthetic display records; wrapper examples are not real-plugin evidence. */
export const builtinPlugin = (
  position = 1,
  value = "3x Osc",
): ProjectPluginReference => ({
  position,
  className: { status: "extracted", value },
  name: { status: "extracted", value },
  vendor: {
    status: "unavailable",
    value: null,
    reason: "PLUGIN_VENDOR_NOT_STORED",
  },
});
export const savedPlugins = (): ProjectPluginReferences => ({
  coverage: "top-level-saved-references",
  items: [
    builtinPlugin(),
    {
      position: 2,
      className: {
        status: "unavailable",
        value: null,
        reason: "PLUGIN_NAME_NOT_STORED",
      },
      name: {
        status: "inferred",
        value: "Sampler",
        method: "sampler-default-for-known-build",
        confidence: "high",
      },
      vendor: {
        status: "unavailable",
        value: null,
        reason: "PLUGIN_VENDOR_NOT_STORED",
      },
    },
    {
      position: 3,
      className: { status: "extracted", value: "Fruity Wrapper" },
      name: { status: "extracted", value: "Synthetic Synth" },
      vendor: { status: "extracted", value: "Synthetic Vendor" },
    },
    builtinPlugin(4),
  ],
});
