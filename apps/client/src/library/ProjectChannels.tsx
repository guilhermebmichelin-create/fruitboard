import { useState } from "react";
import {
  channelLabel,
  type ChannelDetail,
  type ProjectChannel,
} from "./projectDetails";

const BATCH_SIZE = 20;
export function ProjectChannels({
  channels,
}: {
  readonly channels: readonly ProjectChannel[];
}) {
  const [visible, setVisible] = useState(BATCH_SIZE);
  const shown = Math.min(visible, channels.length);
  return (
    <section
      className="project-channels"
      aria-label="Saved channels and instruments"
    >
      <h5>Channels and instruments</h5>
      <p className="project-channels__note">
        Channels are listed in saved parser order. A channel label can be
        renamed; it does not identify its instrument. Instrument coverage is
        limited to verified built-in classes and excludes mixer effects and
        nested plugins.
      </p>
      {channels.length === 0 ? (
        <p>No channels were stored in this result.</p>
      ) : (
        <>
          <p role="status">
            Showing {shown} of {channels.length} channels.
          </p>
          <ol className="project-channels__list">
            {channels.slice(0, shown).map((channel) => (
              <li key={channel.position}>
                <h6>Channel {channel.position}</h6>
                <dl className="project-details__facts">
                  <div>
                    <dt>Channel label</dt>
                    <dd>
                      <Detail value={channel.name} />
                    </dd>
                  </div>
                  <div>
                    <dt>Instrument</dt>
                    <dd>
                      <Detail value={channel.instrument} />
                    </dd>
                  </div>
                </dl>
              </li>
            ))}
          </ol>
          {channels.length > BATCH_SIZE && (
            <button
              className="library-button library-button--secondary"
              type="button"
              onClick={() =>
                setVisible(
                  shown === channels.length
                    ? BATCH_SIZE
                    : Math.min(visible + BATCH_SIZE, channels.length),
                )
              }
            >
              {shown === channels.length
                ? "Show fewer channels"
                : `Show next ${Math.min(BATCH_SIZE, channels.length - shown)} channels`}
            </button>
          )}
        </>
      )}
    </section>
  );
}

function Detail({ value }: { readonly value: ChannelDetail }) {
  const text =
    value.value === null
      ? value.status === "unsupported"
        ? "Unsupported"
        : "Unavailable"
      : channelLabel(value.value);
  return (
    <>
      <div className="project-details__value">
        <strong>
          <bdi>{text}</bdi>
        </strong>
        <span
          className={`project-details__badge project-details__badge--${value.status}`}
        >
          {value.status[0]?.toUpperCase()}
          {value.status.slice(1)}
        </span>
      </div>
      {value.explanation && <p>{value.explanation}</p>}
      {value.value !== null &&
        value.value.length > 0 &&
        text !== value.value && (
          <p>Control characters are shown as escape codes.</p>
        )}
    </>
  );
}
