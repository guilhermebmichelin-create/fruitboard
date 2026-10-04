import { useState } from "react";
import { channelLabel } from "./projectDetails";
import {
  PLUGIN_REASON_COPY,
  type ProjectPluginDetail,
  type ProjectPluginReferences,
} from "./pluginReferences";

export function PluginField({
  label,
  detail,
}: {
  readonly label: string;
  readonly detail: ProjectPluginDetail;
}) {
  return (
    <div>
      <dt>{label}</dt>
      <dd>
        <span className="project-details__badge">
          {detail.status[0]?.toUpperCase()}
          {detail.status.slice(1)}
        </span>
        {detail.value === null ? (
          <p>{PLUGIN_REASON_COPY[detail.reason]}</p>
        ) : (
          <p className="project-plugins__text">
            <bdi>{channelLabel(detail.value)}</bdi>
          </p>
        )}
        {detail.status === "inferred" && (
          <p>
            High confidence. Default Sampler name for this verified saved build;
            no class name was stored.
          </p>
        )}
      </dd>
    </div>
  );
}
export function ProjectPlugins({
  plugins,
}: {
  readonly plugins: ProjectPluginReferences | undefined;
}) {
  const [visible, setVisible] = useState(20);
  const shown = Math.min(visible, plugins?.items.length ?? 0);
  return (
    <section className="project-plugins" aria-label="Saved plugin references">
      <h5>Plugin references</h5>
      <p className="project-plugins__note">
        These are top-level references saved in the project, in reference order.
        Fruitboard has not checked plugin installation or availability. This
        list may omit nested plugins and does not establish complete plugin
        coverage.
      </p>
      {plugins === undefined ? (
        <p>
          This version of the app did not report plugin references for this
          result.
        </p>
      ) : plugins.items.length === 0 ? (
        <p>
          No top-level plugin references were saved in this result. This does
          not prove the project uses no plugins.
        </p>
      ) : (
        <>
          <p>
            {plugins.items.length} saved plugin{" "}
            {plugins.items.length === 1 ? "reference" : "references"}.
            Duplicates are kept; this is not a count of unique plugins.
          </p>
          <p role="status">
            Showing {shown} of {plugins.items.length} plugin references.
          </p>
          <ol className="project-plugins__list">
            {plugins.items.slice(0, shown).map((item) => (
              <li key={item.position}>
                <h6>Plugin reference {item.position}</h6>
                <dl className="project-details__facts">
                  <PluginField label="Name" detail={item.name} />
                  <PluginField label="Saved class" detail={item.className} />
                  <PluginField label="Vendor" detail={item.vendor} />
                </dl>
              </li>
            ))}
          </ol>
          {plugins.items.length > 20 && (
            <button
              type="button"
              className="button button--secondary"
              onClick={() =>
                setVisible(shown === plugins.items.length ? 20 : shown + 20)
              }
            >
              {shown === plugins.items.length
                ? "Show fewer plugin references"
                : `Show next ${Math.min(20, plugins.items.length - shown)} plugin references`}
            </button>
          )}
        </>
      )}
      <p className="project-plugins__note">
        Control characters in saved text are shown as escape codes. Reference
        positions are not channel numbers or stable plugin IDs.
      </p>
    </section>
  );
}
