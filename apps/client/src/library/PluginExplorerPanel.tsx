import { useEffect, useState } from "react";
import type { LibraryScanAdapter } from "./contracts";
import { channelLabel } from "./projectDetails";
import { ProjectDetailsPanel } from "./ProjectDetailsPanel";
import { PluginField } from "./ProjectPlugins";
import {
  EXPLORER_GAP_COPY,
  type PluginExplorerEntry,
  type PluginExplorerGroup,
  type PluginExplorerResult,
} from "./pluginExplorer";

export function PluginExplorerPanel({
  adapter,
  rootId,
  rootLabel,
  sourceSnapshotId,
}: {
  readonly adapter: LibraryScanAdapter;
  readonly rootId: string;
  readonly rootLabel: string;
  readonly sourceSnapshotId: string;
}) {
  const [open, setOpen] = useState(false);
  const [refresh, setRefresh] = useState(0);
  return (
    <section
      className="library-results plugin-explorer"
      aria-labelledby="plugin-explorer-title"
    >
      <div className="library-results__heading">
        <div>
          <p className="eyebrow">Saved project references</p>
          <h2 id="plugin-explorer-title">Plugin Explorer</h2>
        </div>
        <button
          type="button"
          className="library-button library-button--secondary"
          aria-expanded={open}
          aria-controls="plugin-explorer-body"
          onClick={() => setOpen(!open)}
        >
          {open ? "Close Plugin Explorer" : "Open Plugin Explorer"}
        </button>
      </div>
      <p>
        Find saved plugin names across <bdi>{channelLabel(rootLabel)}</bdi>.
        Matching counts refer to Library file entries; copies and repeated
        references are kept.
      </p>
      {open && (
        <div id="plugin-explorer-body">
          <button
            type="button"
            className="library-button library-button--secondary"
            onClick={() => setRefresh(refresh + 1)}
          >
            Refresh Plugin Explorer
          </button>
          <ExplorerResult
            key={`${rootId}:${sourceSnapshotId}:${refresh}`}
            adapter={adapter}
            rootId={rootId}
          />
        </div>
      )}
    </section>
  );
}

function ExplorerResult({
  adapter,
  rootId,
}: {
  readonly adapter: LibraryScanAdapter;
  readonly rootId: string;
}) {
  const [response, setResponse] = useState<{
    readonly adapter: LibraryScanAdapter;
    readonly data: PluginExplorerResult | null;
  } | null>(null);
  useEffect(() => {
    let disposed = false;
    const pending = Promise.resolve().then(
      () =>
        adapter.getPluginExplorer?.(rootId) ??
        ({ rootId, state: "disabled" } as const),
    );
    void pending.then(
      (data) => {
        if (!disposed) setResponse({ adapter, data });
      },
      () => {
        if (!disposed) setResponse({ adapter, data: null });
      },
    );
    return () => {
      disposed = true;
    };
  }, [adapter, rootId]);
  if (response === null || response.adapter !== adapter)
    return (
      <p role="status" aria-busy="true">
        Loading saved plugin references…
      </p>
    );
  if (response.data === null)
    return (
      <p role="alert">
        Plugin Explorer could not be read. Refresh to try again.
      </p>
    );
  if (response.data.rootId !== rootId)
    return (
      <p role="alert">
        Plugin Explorer could not be read. Refresh to try again.
      </p>
    );
  if (response.data.state === "limited")
    return (
      <p role="status">
        This folder exceeds the supported Explorer query budget. No partial
        totals or matching lists are shown. Choose a smaller scan root.
      </p>
    );
  if (response.data.state !== "ready")
    return (
      <p role="status">
        Plugin Explorer is unavailable for this folder or app version. An
        enabled local folder with saved analysis is required.
      </p>
    );
  return <ReadyExplorer key={rootId} data={response.data} adapter={adapter} />;
}

function ReadyExplorer({
  data,
  adapter,
}: {
  readonly data: Extract<PluginExplorerResult, { state: "ready" }>;
  readonly adapter: LibraryScanAdapter;
}) {
  const [search, setSearch] = useState("");
  const [visible, setVisible] = useState(20);
  const [gapVisible, setGapVisible] = useState(20);
  const term = search.trim().toLowerCase();
  const filtered = data.groups.filter((group) =>
    [group.plugin.name, group.plugin.className, group.plugin.vendor].some(
      (field) => field.value?.toLowerCase().includes(term),
    ),
  );
  const entries = new Map(
    data.entries.map((entry) => [entry.locationId, entry]),
  );
  const current = data.entries.filter(
    (entry) => entry.metadataState === "current",
  ).length;
  const gaps = data.entries.filter(
    (entry) =>
      entry.metadataState !== "current" || entry.unnamedReferenceCount > 0,
  );
  return (
    <>
      <p role="status">
        {data.groups.length} exact saved plugin groups across {current} of{" "}
        {data.entries.length} Library entries. {data.referenceCount} references,
        including {data.unnamedReferenceCount} without a usable name.
      </p>
      <p>
        Only current saved top-level references are included. Nested plugins may
        be omitted. Plugin installation and availability have not been checked.
        Refresh reads saved results; it does not analyze files.
      </p>
      {data.entries.length === 0 ? (
        <p>No committed Library entries in this folder yet.</p>
      ) : (
        <>
          <label className="plugin-explorer__search">
            Search saved plugin names, classes or vendors
            <input
              type="search"
              maxLength={4095}
              value={search}
              onChange={(event) => {
                setSearch(event.target.value);
                setVisible(20);
              }}
            />
          </label>
          <p role="status">
            Showing {Math.min(visible, filtered.length)} of {filtered.length}{" "}
            matching groups.
          </p>
          {filtered.length === 0 && (
            <p>
              {data.groups.length === 0
                ? "No named plugin references in current saved results. This does not prove these projects use no plugins."
                : "No saved plugin groups match this search."}
            </p>
          )}
          <ul
            className="plugin-explorer__groups"
            aria-label="Saved plugin groups"
          >
            {filtered.slice(0, visible).map((group) => (
              <PluginGroup
                key={JSON.stringify(group.plugin)}
                group={group}
                entries={entries}
                adapter={adapter}
                id={data.groups.indexOf(group)}
              />
            ))}
          </ul>
          {filtered.length > visible && (
            <button
              className="library-button library-button--secondary"
              type="button"
              onClick={() => setVisible(visible + 20)}
            >
              Show more plugin groups
            </button>
          )}
          <section
            aria-label="Plugin analysis coverage"
            className="plugin-explorer__coverage"
          >
            <h3>Coverage gaps</h3>
            {gaps.length === 0 ? (
              <p>
                Every entry has current saved plugin-reference information. This
                is still top-level saved coverage, not complete plugin
                discovery.
              </p>
            ) : (
              <>
                <p>
                  {gaps.length} entries have missing or incomplete plugin
                  information. Missing results and unnamed references are not
                  counted as named matches.
                </p>
                <ul>
                  {gaps.slice(0, gapVisible).map((entry) => (
                    <li key={entry.locationId}>
                      <bdi>{channelLabel(entry.relativePath)}</bdi>:{" "}
                      {EXPLORER_GAP_COPY[entry.metadataState]}
                      {entry.unnamedReferenceCount > 0 &&
                        `; ${entry.unnamedReferenceCount} unnamed references`}
                    </li>
                  ))}
                </ul>
                <p>
                  Showing {Math.min(gapVisible, gaps.length)} of {gaps.length}{" "}
                  coverage gaps.
                </p>
                {gaps.length > gapVisible && (
                  <button
                    type="button"
                    className="library-button library-button--secondary"
                    onClick={() => setGapVisible(gapVisible + 20)}
                  >
                    Show more coverage gaps
                  </button>
                )}
              </>
            )}
          </section>
        </>
      )}
      <p className="project-plugins__note">
        Exact spelling and saved extraction or inference are kept separate.
        Control characters are displayed as escape codes. Counts describe the
        saved results when this view was read.
      </p>
    </>
  );
}

function PluginGroup({
  group,
  entries,
  adapter,
  id,
}: {
  readonly group: PluginExplorerGroup;
  readonly entries: ReadonlyMap<string, PluginExplorerEntry>;
  readonly adapter: LibraryScanAdapter;
  readonly id: number;
}) {
  const [open, setOpen] = useState(false);
  const [visible, setVisible] = useState(20);
  return (
    <li className="library-record-item">
      <h3>
        <bdi>{channelLabel(group.plugin.name.value ?? "")}</bdi>
      </h3>
      <p>
        {group.matches.length} matching{" "}
        {group.matches.length === 1 ? "entry" : "entries"} ·{" "}
        {group.referenceCount} saved{" "}
        {group.referenceCount === 1 ? "reference" : "references"}
      </p>
      <dl className="project-details__facts">
        <PluginField label="Name" detail={group.plugin.name} />
        <PluginField label="Saved class" detail={group.plugin.className} />
        <PluginField label="Vendor" detail={group.plugin.vendor} />
      </dl>
      <button
        type="button"
        className="library-button library-button--secondary"
        aria-expanded={open}
        aria-controls={`plugin-matches-${id}`}
        onClick={() => setOpen(!open)}
      >
        {open ? "Hide" : "Show"} matching entries for{" "}
        {channelLabel(group.plugin.name.value ?? "")}
      </button>
      {open && (
        <div id={`plugin-matches-${id}`}>
          <ul
            className="plugin-explorer__matches"
            aria-label={`Matching entries for ${channelLabel(group.plugin.name.value ?? "")}`}
          >
            {group.matches.slice(0, visible).map((match) => {
              const entry = entries.get(match.locationId);
              return (
                entry && (
                  <li key={entry.locationId}>
                    <h4>
                      <bdi>{channelLabel(entry.fileName)}</bdi>
                    </h4>
                    <p>
                      <bdi>{channelLabel(entry.relativePath)}</bdi> ·{" "}
                      {match.referenceCount} saved references
                    </p>
                    <ProjectDetailsPanel
                      adapter={adapter}
                      record={entry}
                      idPrefix={`explorer-details-${id}`}
                    />
                  </li>
                )
              );
            })}
          </ul>
          <p>
            Showing {Math.min(visible, group.matches.length)} of{" "}
            {group.matches.length} matching entries.
          </p>
          {group.matches.length > visible && (
            <button
              type="button"
              className="library-button library-button--secondary"
              onClick={() => setVisible(visible + 20)}
            >
              Show more matching entries
            </button>
          )}
        </div>
      )}
    </li>
  );
}
