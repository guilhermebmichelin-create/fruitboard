import { useEffect, useState } from "react";
import type { LibraryScanAdapter, PublishedFileLocation } from "./contracts";
import { ProjectChannels } from "./ProjectChannels";
import { ProjectSamples } from "./ProjectSamples";
import { ProjectPlugins } from "./ProjectPlugins";
import { ProjectAnalysisStatus } from "./ProjectAnalysisStatus";
import { ProjectAnalysisAction } from "./ProjectAnalysisAction";
import {
  FACT_LABELS,
  FACT_NOTES,
  formatProjectFact,
  type ProjectDetails,
} from "./projectDetails";

type State =
  | { readonly kind: "loading" }
  | { readonly kind: "error" }
  | { readonly kind: "ready"; readonly details: ProjectDetails };

/** Parent keys by scan snapshot and row fingerprint; new results clear facts. */
export function ProjectDetailsPanel({
  adapter,
  record,
  sourceEligible = true,
  idPrefix = "project-details",
}: {
  readonly adapter: LibraryScanAdapter;
  readonly record: PublishedFileLocation;
  readonly sourceEligible?: boolean;
  readonly idPrefix?: string;
}) {
  const [open, setOpen] = useState(false);
  const [refresh, setRefresh] = useState(0);
  const [state, setState] = useState<State>({ kind: "loading" });
  const { rootId, locationId, byteSize, modifiedAt, presence } = record;
  useEffect(() => {
    if (!open) return;
    let disposed = false;
    // The request carries only row identity/fingerprint; names and paths never
    // become arguments. Older adapters explicitly show metadata unavailable.
    const requestRecord = { rootId, locationId, byteSize, modifiedAt };
    const pending =
      presence === "missing" || !sourceEligible
        ? Promise.resolve<ProjectDetails>({
            rootId,
            locationId,
            state: "no_current",
            analysis: { state: "not_current", attempts: null, reason: null },
          })
        : Promise.resolve().then<ProjectDetails>(
            () =>
              adapter.getProjectDetails?.(requestRecord) ?? {
                rootId,
                locationId,
                state: "disabled",
              },
          );
    void pending.then(
      (details) => {
        if (!disposed) setState({ kind: "ready", details });
      },
      () => {
        if (!disposed) setState({ kind: "error" });
      },
    );
    return () => {
      disposed = true;
    };
  }, [
    adapter,
    open,
    refresh,
    rootId,
    locationId,
    byteSize,
    modifiedAt,
    presence,
    sourceEligible,
  ]);

  const id = `${idPrefix}-${locationId}`;
  return (
    <div className="project-details">
      <button
        aria-controls={id}
        aria-expanded={open}
        aria-label={`Project details ${record.fileName}`}
        className="library-button library-button--secondary"
        type="button"
        onClick={() => {
          setState({ kind: "loading" });
          setOpen(!open);
        }}
      >
        {open ? "Hide project details" : "Project details"}
      </button>
      {open && (
        <section
          id={id}
          aria-label={`Saved project details ${record.fileName}`}
          className="project-details__body"
        >
          <div className="project-details__heading">
            <h4>Saved project details</h4>
            <button
              className="library-button library-button--secondary"
              type="button"
              aria-label={`Refresh project details ${record.fileName}`}
              disabled={state.kind === "loading"}
              onClick={() => {
                setState({ kind: "loading" });
                setRefresh(refresh + 1);
              }}
            >
              Refresh details
            </button>
          </div>
          {state.kind === "loading" && (
            <p role="status">Loading saved project details…</p>
          )}
          {state.kind === "error" && (
            <p role="alert">
              Project details could not be read safely. Refresh details to try
              again.
            </p>
          )}
          {state.kind === "ready" && state.details.state !== "disabled" && (
            <ProjectAnalysisAction
              key={`${rootId}:${locationId}:${byteSize}:${modifiedAt}:${sourceEligible}`}
              adapter={adapter}
              record={record}
              request={state.details.analysisRequest}
              hasFacts={state.details.state === "available"}
              failed={
                state.details.analysis?.state === "failed" ||
                state.details.analysis?.state === "cancelled"
              }
              onQueued={() => {
                setRefresh((count) => count + 1);
              }}
            />
          )}
          {state.kind === "ready" && state.details.state !== "disabled" && (
            <ProjectAnalysisStatus
              analysis={state.details.analysis}
              hasFacts={state.details.state === "available"}
            />
          )}
          {state.kind === "ready" &&
            (state.details.state === "disabled" ? (
              <p>
                Project details are unavailable in this version of Fruitboard.
              </p>
            ) : state.details.state === "no_current" ? (
              <p>
                No current saved project details. The file may be missing,
                changed, not yet analyzed, or unsupported.
              </p>
            ) : (
              <>
                <p className="project-details__intro">
                  Saved parser result ·{" "}
                  {state.details.outcome === "partial"
                    ? "Partial coverage"
                    : "Supported fields read"}
                  . Matched the scanned file when these details were read.
                  Refresh details to check again.
                </p>
                {state.details.warnings?.includes("unverified_events") && (
                  <p>
                    Some saved events are not understood. Supported fields are
                    still shown.
                  </p>
                )}
                <dl className="project-details__facts">
                  {state.details.facts.map((fact) => (
                    <div key={fact.key} data-fact-status={fact.status}>
                      <dt>{FACT_LABELS[fact.key]}</dt>
                      <dd>
                        <div className="project-details__value">
                          <strong>{formatProjectFact(fact)}</strong>
                          <span
                            className={`project-details__badge project-details__badge--${fact.status}`}
                          >
                            {fact.status[0]?.toUpperCase()}
                            {fact.status.slice(1)}
                          </span>
                        </div>
                        {fact.explanation && <p>{fact.explanation}</p>}
                        {FACT_NOTES[fact.key] && <p>{FACT_NOTES[fact.key]}</p>}
                      </dd>
                    </div>
                  ))}
                </dl>
                <ProjectChannels channels={state.details.channels} />
                <ProjectSamples
                  key={state.details.snapshotId}
                  samples={state.details.sampleReferences}
                />
                <ProjectPlugins
                  key={state.details.snapshotId}
                  plugins={state.details.pluginReferences}
                />
              </>
            ))}
        </section>
      )}
    </div>
  );
}
