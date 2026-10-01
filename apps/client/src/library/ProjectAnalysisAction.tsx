import { useEffect, useRef, useState } from "react";
import {
  LibraryAdapterError,
  type LibraryScanAdapter,
  type PublishedFileLocation,
} from "./contracts";
import {
  ANALYSIS_REQUEST_NOTES,
  type ProjectAnalysisRequest,
} from "./projectAnalysisRequest";

export function ProjectAnalysisAction({
  adapter,
  record,
  request,
  hasFacts,
  failed,
  onQueued,
}: {
  readonly adapter: LibraryScanAdapter;
  readonly record: PublishedFileLocation;
  readonly request: ProjectAnalysisRequest | undefined;
  readonly hasFacts: boolean;
  readonly failed: boolean;
  readonly onQueued: () => void;
}) {
  const [message, setMessage] = useState<string | null>(null);
  const [blocked, setBlocked] = useState(false);
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const active = useRef(true);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  if (request === undefined || adapter.requestProjectAnalysis === undefined)
    return null;
  const label = failed
    ? "Retry analysis"
    : hasFacts
      ? "Analyze again"
      : "Analyze project";
  const submit = async () => {
    if (
      busy.current ||
      blocked ||
      request.state !== "ready" ||
      adapter.requestProjectAnalysis === undefined
    )
      return;
    busy.current = true;
    setPending(true);
    setMessage("Requesting analysis…");
    const requestRecord = {
      rootId: record.rootId,
      locationId: record.locationId,
      byteSize: record.byteSize,
      modifiedAt: record.modifiedAt,
    };
    try {
      const result = await adapter.requestProjectAnalysis(
        requestRecord,
        request.requestKey,
      );
      if (!active.current) return;
      setBlocked(true);
      if (result.state === "queued") {
        setMessage(
          hasFacts
            ? "Analysis requested. Saved facts remain visible while another attempt runs."
            : "Analysis requested. Refresh details to check for a result.",
        );
        onQueued();
      } else
        setMessage(
          result.state === "blocked"
            ? ANALYSIS_REQUEST_NOTES[result.reason]
            : "Analysis is unavailable in this version of Fruitboard.",
        );
    } catch (error) {
      if (!active.current) return;
      setBlocked(true);
      setMessage(
        error instanceof LibraryAdapterError &&
          ["conflict", "not_found"].includes(error.code)
          ? "The file or analysis state changed. Refresh details before trying again."
          : "Analysis could not be requested safely. Refresh details before trying again.",
      );
    } finally {
      busy.current = false;
      if (active.current) setPending(false);
    }
  };
  return (
    <div className="project-analysis-action">
      <button
        className="library-button library-button--secondary"
        type="button"
        aria-label={`${label} ${record.fileName}`}
        disabled={pending || blocked || request.state !== "ready"}
        onClick={() => {
          void submit();
        }}
      >
        {label}
      </button>
      <p>
        A new request keeps the attempt count for this unchanged file.
        Fruitboard reads the project; it does not modify its FLP bytes.
      </p>
      {request.state !== "ready" && (
        <p>{ANALYSIS_REQUEST_NOTES[request.state]}</p>
      )}
      {message && <p role="status">{message}</p>}
    </div>
  );
}
