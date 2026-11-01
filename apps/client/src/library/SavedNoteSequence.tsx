import { useState } from "react";
import type { ProjectChannel } from "./projectDetails";
import { channelLabel } from "./projectDetails";
import {
  NOTE_SEQUENCE_REASONS,
  type SavedNoteSequence as Sequence,
} from "./projectNoteSequences";

const BATCH_SIZE = 64;
export function SavedNoteSequence({
  patternId,
  sequence,
  channels,
  reason,
}: {
  readonly patternId: number;
  readonly sequence?: Sequence | undefined;
  readonly channels: readonly ProjectChannel[];
  readonly reason?: keyof typeof NOTE_SEQUENCE_REASONS | undefined;
}) {
  const [visible, setVisible] = useState(BATCH_SIZE);
  const records =
    sequence?.status === "extracted" ? sequence.records : undefined;
  const shown = Math.min(visible, records?.length ?? 0);
  const missingReason =
    sequence?.status === "extracted" ? reason : (sequence?.reason ?? reason);
  return (
    <details className="project-note-sequence">
      <summary>Saved note sequence for pattern {patternId}</summary>
      {!records ? (
        <p>{NOTE_SEQUENCE_REASONS[missingReason ?? "not_saved"]}</p>
      ) : (
        <>
          <p>
            Pattern-local ticks, 96 PPQ. Key and velocity are raw saved numbers.
            These details cover qualified ordinary records only. Saved note
            counts can include other record kinds.
          </p>
          <p role="status">
            Showing {shown} of {records.length} saved records.
          </p>
          {channels.map((channel, index) => {
            // Disclosure selects source records before grouping; ordering and
            // duplicate ordinals remain intact within each channel.
            const notes = records
              .slice(0, shown)
              .filter((note) => note.channelIndex === index);
            if (notes.length === 0) return null;
            const name = channel.name.value;
            const label =
              name === null
                ? "No saved channel name"
                : name.length === 0
                  ? "Empty saved channel name"
                  : channelLabel(name);
            return (
              <section
                key={channel.position}
                aria-label={`Pattern ${patternId}, channel ${channel.position} note records`}
              >
                <p>
                  <strong>
                    Channel {channel.position} · <bdi>{label}</bdi>
                  </strong>
                </p>
                {name !== null && label !== name && name.length > 0 && (
                  <p>Control characters are shown as escape codes.</p>
                )}
                <ol className="project-note-sequence__records">
                  {notes.map((note) => (
                    <li key={note.ordinal} value={note.ordinal + 1}>
                      <p>
                        <strong>Saved record {note.ordinal + 1}</strong>
                      </p>
                      <dl className="project-details__facts">
                        <div>
                          <dt>Saved key</dt>
                          <dd>{note.savedKey}</dd>
                        </div>
                        <div>
                          <dt>Start tick</dt>
                          <dd>{note.startTick}</dd>
                        </div>
                        <div>
                          <dt>Duration ticks</dt>
                          <dd>{note.durationTick}</dd>
                        </div>
                        <div>
                          <dt>Stored velocity byte</dt>
                          <dd>{note.storedVelocityByte}</dd>
                        </div>
                      </dl>
                    </li>
                  ))}
                </ol>
              </section>
            );
          })}
          {records.length > BATCH_SIZE && (
            <button
              className="library-button library-button--secondary"
              type="button"
              aria-label={`${shown === records.length ? "Show fewer notes" : "Show more notes"} for pattern ${patternId}`}
              onClick={() =>
                setVisible(
                  shown === records.length
                    ? BATCH_SIZE
                    : Math.min(visible + BATCH_SIZE, records.length),
                )
              }
            >
              {shown === records.length
                ? "Show fewer notes"
                : `Show next ${Math.min(BATCH_SIZE, records.length - shown)} notes`}
            </button>
          )}
        </>
      )}
    </details>
  );
}
