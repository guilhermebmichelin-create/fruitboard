import { useState } from "react";
import { channelLabel } from "./projectDetails";
import {
  PATTERN_REASONS,
  type ProjectPatterns as Patterns,
} from "./projectPatterns";
import {
  NOTE_COUNT_REASONS,
  type ProjectPatternNotes,
} from "./projectPatternNotes";

const BATCH_SIZE = 20;
export function ProjectPatterns({
  patterns,
  notes,
}: {
  readonly patterns?: Patterns | undefined;
  readonly notes?: ProjectPatternNotes | undefined;
}) {
  const [visible, setVisible] = useState(BATCH_SIZE);
  if (!patterns || patterns.state !== "available") {
    const reason = patterns?.reason ?? "not_saved";
    return (
      <section className="project-channels" aria-label="Saved patterns">
        <h5>Saved patterns</h5>
        <p>{PATTERN_REASONS[reason]}</p>
      </section>
    );
  }
  const shown = Math.min(visible, patterns.count);
  return (
    <section className="project-channels" aria-label="Saved patterns">
      <h5>Saved patterns</h5>
      <p className="project-channels__note">
        Counts describe distinct stored patterns, not playlist placements. Saved
        pattern IDs can have gaps. Saved note records include slide notes and
        steps, even entries that may not sound. Reusing a pattern in the
        Playlist does not increase its saved note-record count.
      </p>
      <p role="status">
        Showing {shown} of {patterns.count} saved patterns.
      </p>
      <ol className="project-channels__list">
        {patterns.items.slice(0, shown).map((pattern, index) => {
          const noteCount =
            notes?.state === "available"
              ? notes.items[index]?.noteCount
              : undefined;
          const value = pattern.name.value;
          const text =
            value === null
              ? "No saved name"
              : value.length === 0
                ? "Empty saved name"
                : channelLabel(value);
          return (
            <li key={pattern.patternId}>
              <h6>Pattern {pattern.patternId}</h6>
              <dl className="project-details__facts">
                <div>
                  <dt>Saved name</dt>
                  <dd>
                    <div className="project-details__value">
                      <strong>
                        <bdi>{text}</bdi>
                      </strong>
                      <span
                        className={`project-details__badge project-details__badge--${pattern.name.status}`}
                      >
                        {pattern.name.status === "extracted"
                          ? "Extracted"
                          : "Unavailable"}
                      </span>
                    </div>
                    {value !== null && value.length > 0 && text !== value && (
                      <p>Control characters are shown as escape codes.</p>
                    )}
                  </dd>
                </div>
                <div>
                  <dt>Saved note records</dt>
                  <dd>
                    {noteCount?.status === "extracted" ? (
                      <strong>{noteCount.value.toLocaleString("en-US")}</strong>
                    ) : (
                      <p>
                        {
                          NOTE_COUNT_REASONS[
                            noteCount?.reason ??
                              (notes?.state !== "available"
                                ? notes?.reason
                                : undefined) ??
                              "not_saved"
                          ]
                        }
                      </p>
                    )}
                  </dd>
                </div>
              </dl>
            </li>
          );
        })}
      </ol>
      {patterns.count > BATCH_SIZE && (
        <button
          className="library-button library-button--secondary"
          type="button"
          onClick={() =>
            setVisible(
              shown === patterns.count
                ? BATCH_SIZE
                : Math.min(visible + BATCH_SIZE, patterns.count),
            )
          }
        >
          {shown === patterns.count
            ? "Show fewer patterns"
            : `Show next ${Math.min(BATCH_SIZE, patterns.count - shown)} patterns`}
        </button>
      )}
    </section>
  );
}
