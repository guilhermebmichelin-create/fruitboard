import { useState } from "react";
import { channelLabel } from "./projectDetails";
import type { ProjectSampleReference } from "./sampleReferences";
import { useSamplePresence, type SampleCheckView } from "./useSamplePresence";
import { SamplePresenceControls } from "./SamplePresenceControls";
import { UNCHECKED_COPY } from "./samplePresence";

const BATCH_SIZE = 20;
export function ProjectSamples({
  samples,
  checkView,
}: {
  readonly samples: readonly ProjectSampleReference[] | undefined;
  readonly checkView?: SampleCheckView;
}) {
  const check = useSamplePresence(checkView, samples);
  const [visible, setVisible] = useState(BATCH_SIZE);
  const shown = Math.min(visible, samples?.length ?? 0);
  const savedCount =
    samples?.filter((sample) => sample.status === "extracted").length ?? 0;
  return (
    <section className="project-samples" aria-label="Saved sample references">
      <h5>Sample references</h5>
      <p className="project-samples__note">
        These are references saved per channel, in parser order. Fruitboard has
        not located these files through FL Studio's search folders. A saved
        reference does not prove a file is present or missing.
      </p>
      {checkView && (
        <SamplePresenceControls
          state={check.state}
          canCheck={check.canCheck}
          available={check.available}
          folder={checkView.record.rootDisplayName}
          onCheck={() => {
            void check.check();
          }}
          onCancel={check.cancel}
        />
      )}
      {samples === undefined ? (
        <p>
          This version of the app did not report sample references for this
          result. Analyze the project again to obtain saved references.
        </p>
      ) : samples.length === 0 ? (
        <p>
          No channels were stored in this result, so no per-channel references
          are shown.
        </p>
      ) : (
        <>
          <p>
            {savedCount} of {samples.length} channels have a saved sample
            reference.
          </p>
          {savedCount === 0 && (
            <p>No sample references were stored for the reported channels.</p>
          )}
          <p role="status">
            Showing {shown} of {samples.length} channels for sample references.
          </p>
          <ol className="project-samples__list">
            {samples.slice(0, shown).map((sample) => {
              const text =
                sample.value === null ? null : channelLabel(sample.value);
              const result =
                check.state.state === "complete"
                  ? check.state.report.channels[sample.position - 1]
                  : undefined;
              return (
                <li key={sample.position}>
                  <h6>Sample reference for channel {sample.position}</h6>
                  {text === null ? (
                    <p>No sample reference was stored for this channel.</p>
                  ) : (
                    <>
                      <p className="project-samples__reference">
                        <bdi>{text}</bdi>
                      </p>
                      <span className="project-details__badge project-details__badge--extracted">
                        Extracted ·{" "}
                        {result?.status === "present"
                          ? "Present when checked"
                          : result?.status === "not_found"
                            ? "Not found at saved path when checked"
                            : "Location not checked"}
                      </span>
                      {result?.status === "not_checked" && (
                        <p>Not checked. {UNCHECKED_COPY[result.reason]}</p>
                      )}
                      {text !== sample.value && (
                        <p>Control characters are shown as escape codes.</p>
                      )}
                    </>
                  )}
                </li>
              );
            })}
          </ol>
          {samples.length > BATCH_SIZE && (
            <button
              className="library-button library-button--secondary"
              type="button"
              onClick={() =>
                setVisible(
                  shown === samples.length
                    ? BATCH_SIZE
                    : Math.min(visible + BATCH_SIZE, samples.length),
                )
              }
            >
              {shown === samples.length
                ? "Show fewer sample references"
                : `Show next ${Math.min(BATCH_SIZE, samples.length - shown)} channels for sample references`}
            </button>
          )}
        </>
      )}
    </section>
  );
}
