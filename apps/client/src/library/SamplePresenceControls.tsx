import { UNCHECKED_COPY, type SampleCheckResult } from "./samplePresence";
import { useEffect, useRef } from "react";
export function SamplePresenceControls({
  state,
  canCheck,
  available,
  folder,
  onCheck,
  onCancel,
}: {
  readonly state:
    { readonly state: "idle" | "checking" | "error" } | SampleCheckResult;
  readonly canCheck: boolean;
  readonly available: boolean;
  readonly folder: string;
  readonly onCheck: () => void;
  readonly onCancel: () => void;
}) {
  const checkButton = useRef<HTMLButtonElement>(null);
  const restoreFocus = useRef(false);
  useEffect(() => {
    if (restoreFocus.current && state.state !== "checking") {
      restoreFocus.current = false;
      checkButton.current?.focus();
    }
  }, [state.state]);
  const copy = () => {
    switch (state.state) {
      case "idle":
        return "Only an explicit check inspects saved paths.";
      case "checking":
        return "Checking saved samples…";
      case "error":
      case "failed":
        return "Samples could not be checked safely. Try again.";
      case "disabled":
        return "Sample checks are unavailable in this version of Fruitboard.";
      case "busy":
        return "Another sample check is still finishing. Try again shortly.";
      case "stale":
        return "The project, folder, or saved details changed. Refresh details before checking again.";
      case "cancelled":
        return "Sample check cancelled. No result is shown.";
      case "deadline":
        return "The sample check reached its time limit. Try again shortly.";
      case "no_references":
        return "There are no saved sample references to check.";
      case "references_unavailable":
        return "Analyze the project to obtain saved sample references.";
      case "unavailable":
        return `Samples were not checked. ${UNCHECKED_COPY[state.reason]}`;
      case "complete":
        return `Checked ${state.report.channels.length} channel references at ${new Date(state.report.checkedAt).toLocaleString()}. Results describe when this check ran.`;
    }
  };
  return (
    <div className="sample-presence">
      <p>
        Check exact saved absolute paths inside <bdi>{folder}</bdi>. Relative
        paths, placeholders, other folders, and linked or offline locations
        remain unchecked. This reads file metadata only; it does not read or
        play audio.
      </p>
      <div className="sample-presence__actions">
        <button
          ref={checkButton}
          type="button"
          className="library-button library-button--secondary"
          disabled={!canCheck || state.state === "checking"}
          onClick={onCheck}
        >
          {state.state === "complete"
            ? "Check saved samples again"
            : "Check saved samples"}
        </button>
        {state.state === "checking" && (
          <button
            type="button"
            className="library-button library-button--secondary"
            onClick={() => {
              restoreFocus.current = true;
              onCancel();
            }}
          >
            Cancel sample check
          </button>
        )}
      </div>
      {!available && (
        <p>Sample checks are unavailable in this version of Fruitboard.</p>
      )}
      <p
        role={
          state.state === "error" || state.state === "failed"
            ? "alert"
            : "status"
        }
      >
        {copy()}
      </p>
    </div>
  );
}
