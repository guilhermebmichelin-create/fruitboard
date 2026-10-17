import { useState } from "react";
import { channelLabel } from "./projectDetails";
import { MIXER_REASONS, type ProjectMixerInserts } from "./projectMixerInserts";

const BATCH_SIZE = 20;
export function SavedMixerInserts({
  mixer,
}: {
  readonly mixer?: ProjectMixerInserts | undefined;
}) {
  const [visible, setVisible] = useState(BATCH_SIZE);
  if (!mixer || mixer.state !== "available")
    return (
      <section className="project-channels" aria-label="Saved mixer inserts">
        <h5>Saved mixer inserts</h5>
        <p>{MIXER_REASONS[mixer?.reason ?? "not_saved"]}</p>
      </section>
    );
  const shown = Math.min(visible, mixer.count);
  return (
    <section className="project-channels" aria-label="Saved mixer inserts">
      <h5>Saved mixer inserts</h5>
      <p className="project-channels__note">
        These are explicit saved records. The count does not show which inserts
        are used or audible. Saved IDs are positions in this save, not stable FL
        Studio track numbers. Master and Current are excluded.
      </p>
      <p role="status">
        Showing {shown} of {mixer.count} saved mixer inserts.
      </p>
      <ol className="project-channels__list">
        {mixer.items.slice(0, shown).map((insert) => {
          const value =
            insert.name.status === "extracted" ? insert.name.value : null;
          const text = value === null ? "No saved name" : channelLabel(value);
          return (
            <li key={insert.savedInsertId}>
              <h6>Saved insert ID: {insert.savedInsertId}</h6>
              <dl className="project-details__facts">
                <div>
                  <dt>Saved name</dt>
                  <dd>
                    <div className="project-details__value">
                      <strong>
                        <bdi>{text}</bdi>
                      </strong>
                      <span
                        className={`project-details__badge project-details__badge--${insert.name.status}`}
                      >
                        {insert.name.status === "extracted"
                          ? "Extracted"
                          : "Unavailable"}
                      </span>
                    </div>
                    {value === null && <p>This record has no stored name.</p>}
                    {value !== null && text !== value && (
                      <p>Control characters are shown as escape codes.</p>
                    )}
                  </dd>
                </div>
              </dl>
            </li>
          );
        })}
      </ol>
      {mixer.count > BATCH_SIZE && (
        <button
          className="library-button library-button--secondary"
          type="button"
          onClick={() =>
            setVisible(
              shown === mixer.count
                ? BATCH_SIZE
                : Math.min(visible + BATCH_SIZE, mixer.count),
            )
          }
        >
          {shown === mixer.count
            ? "Show fewer mixer inserts"
            : `Show next ${Math.min(BATCH_SIZE, mixer.count - shown)} mixer inserts`}
        </button>
      )}
    </section>
  );
}
