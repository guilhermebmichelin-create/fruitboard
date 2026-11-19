import type { ProjectAutomationCounts } from "./projectDetails";

const explanations = {
  unverified_build:
    "Automation counts are unavailable for this saved FL Studio build.",
  unverified_layout:
    "Automation counts are unavailable for this saved project layout.",
  not_saved: "This analysis did not save automation counts.",
};

export function SavedAutomationCounts({
  counts,
}: {
  readonly counts?: ProjectAutomationCounts | undefined;
}) {
  return (
    <section className="project-channels" aria-label="Saved automation">
      <h5>Saved automation</h5>
      {counts?.state === "available" ? (
        <>
          <dl className="project-details__facts">
            <div>
              <dt>Saved Automation Clips</dt>
              <dd>{counts.objectCount}</dd>
            </div>
            <div>
              <dt>Editable control points</dt>
              <dd>{counts.pointCount}</dd>
            </div>
          </dl>
          <p className="project-channels__note">
            Totals include saved clips without a Playlist placement.
          </p>
        </>
      ) : (
        <p>{explanations[counts?.state ?? "not_saved"]}</p>
      )}
    </section>
  );
}
