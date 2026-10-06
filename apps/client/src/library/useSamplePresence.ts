import { useEffect, useRef, useState } from "react";
import type { LibraryScanAdapter, PublishedFileLocation } from "./contracts";
import type { ProjectSampleReference } from "./sampleReferences";
import type { SampleCheckRequest, SampleCheckResult } from "./samplePresence";

export interface SampleCheckView {
  readonly adapter: LibraryScanAdapter;
  readonly record: PublishedFileLocation;
  readonly snapshotId: string;
}
type State =
  { readonly state: "idle" | "checking" | "error" } | SampleCheckResult;
export function useSamplePresence(
  view: SampleCheckView | undefined,
  samples: readonly ProjectSampleReference[] | undefined,
) {
  const active = useRef<SampleCheckRequest | null>(null);
  const generation = useRef(0);
  const adapter = view?.adapter;
  const rootId = view?.record.rootId;
  const locationId = view?.record.locationId;
  const byteSize = view?.record.byteSize;
  const modifiedAt = view?.record.modifiedAt;
  const snapshotId = view?.snapshotId;
  const presence = view?.record.presence;
  const scope = JSON.stringify([
    rootId,
    locationId,
    byteSize,
    modifiedAt,
    snapshotId,
    presence,
  ]);
  const [stored, setStored] = useState<{
    readonly scope: string;
    readonly adapter: LibraryScanAdapter | undefined;
    readonly value: State;
  } | null>(null);
  const state: State =
    stored?.scope === scope && stored.adapter === adapter
      ? stored.value
      : { state: "idle" };
  const setState = (value: State) => setStored({ scope, adapter, value });
  useEffect(
    () => () => {
      generation.current++;
      const request = active.current;
      active.current = null;
      if (request !== null)
        void adapter?.cancelSampleCheck?.(request).catch(() => {});
    },
    [adapter, rootId, locationId, byteSize, modifiedAt, snapshotId, presence],
  );
  const available =
    adapter?.checkSavedSamples !== undefined &&
    adapter.cancelSampleCheck !== undefined;
  const canCheck =
    available &&
    view?.record.presence === "present" &&
    samples?.some((s) => s.status === "extracted") === true &&
    state.state !== "disabled";
  const check = async () => {
    if (
      !canCheck ||
      view === undefined ||
      samples === undefined ||
      state.state === "checking"
    )
      return;
    const current = ++generation.current;
    let request: SampleCheckRequest;
    try {
      request = {
        schemaVersion: 1,
        rootId: view.record.rootId,
        locationId: view.record.locationId,
        snapshotId: view.snapshotId,
        requestId: crypto.randomUUID(),
        expectedByteSize: view.record.byteSize,
        expectedModifiedAt: view.record.modifiedAt,
      };
    } catch {
      setState({ state: "error" });
      return;
    }
    active.current = request;
    setState({ state: "checking" });
    try {
      const response = await view.adapter.checkSavedSamples?.(request, samples);
      if (generation.current === current)
        setState(response ?? { state: "error" });
    } catch {
      if (generation.current === current) setState({ state: "error" });
    } finally {
      if (generation.current === current) active.current = null;
    }
  };
  const cancel = () => {
    const request = active.current;
    generation.current++;
    active.current = null;
    setState({ state: "cancelled" });
    if (request !== null)
      void adapter?.cancelSampleCheck?.(request).catch(() => {
        // The UI fence already cleared the result; a transport failure cannot
        // revive it. The native worker still owns its deadline/admission gate.
      });
  };
  return { state, canCheck, available, check, cancel };
}
