import { useCallback, useEffect, useRef, useState } from "react";
import {
  PlatformError,
  type PlatformPort,
  type ScanRoot,
  type ScanRootAvailability,
} from "../platform/contracts";

type LoadState =
  | { readonly kind: "loading" }
  | { readonly kind: "error" }
  | { readonly kind: "ready"; readonly roots: readonly ScanRoot[] };

type ActionState =
  | { readonly kind: "idle" }
  | { readonly kind: "working"; readonly action: string }
  | { readonly kind: "notice"; readonly message: string }
  | { readonly kind: "error"; readonly message: string };

type RenameState =
  | { readonly kind: "closed" }
  | { readonly kind: "editing"; readonly id: string; readonly draft: string };

const defaultDisplayName = (path: string): string => {
  const segments = path.split(/[\\/]/).filter((segment) => segment !== "");
  const last = segments[segments.length - 1];
  return last === undefined || last === "" ? "Scan root" : last;
};

const availabilityLabels: Readonly<Record<ScanRootAvailability, string>> = {
  available: "Available",
  unavailable: "Unavailable",
  unknown: "Unknown",
};

const addFailureMessage = (error: unknown): string => {
  if (error instanceof PlatformError && error.code === "conflict") {
    return "That folder is already tracked or overlaps another root.";
  }
  if (error instanceof PlatformError && error.code === "invalid_request") {
    return "That folder could not be used. Choose another folder.";
  }
  return "Fruitboard could not add that folder. Try again.";
};

export function ScanRootsManager({
  platform,
}: {
  readonly platform: PlatformPort;
}) {
  const [loadAttempt, setLoadAttempt] = useState(0);
  const [loadState, setLoadState] = useState<LoadState>({ kind: "loading" });
  const [actionState, setActionState] = useState<ActionState>({ kind: "idle" });
  const [confirmingRemoval, setConfirmingRemoval] = useState<string | null>(
    null,
  );
  const [renameState, setRenameState] = useState<RenameState>({
    kind: "closed",
  });
  const mounted = useRef(true);
  const addButtonReference = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const refresh = useCallback(async () => {
    const roots = await platform.listScanRoots();
    if (mounted.current) {
      setLoadState({ kind: "ready", roots });
    }
  }, [platform]);

  useEffect(() => {
    let current = true;
    void platform.listScanRoots().then(
      (roots) => {
        if (current) {
          setLoadState({ kind: "ready", roots });
        }
      },
      () => {
        if (current) {
          setLoadState({ kind: "error" });
        }
      },
    );

    return () => {
      current = false;
    };
  }, [loadAttempt, platform]);

  const retry = () => {
    setActionState({ kind: "idle" });
    setConfirmingRemoval(null);
    setRenameState({ kind: "closed" });
    setLoadState({ kind: "loading" });
    setLoadAttempt((attempt) => attempt + 1);
  };

  const refreshAfterMutation = async (notice: string) => {
    try {
      await refresh();
    } catch {
      if (mounted.current) {
        // The mutation already succeeded: report the stale list honestly
        // instead of claiming nothing changed.
        setActionState({
          kind: "error",
          message:
            "Saved, but the list could not be refreshed. Reopen Preferences to confirm.",
        });
      }
      return;
    }
    if (mounted.current) {
      setActionState({ kind: "notice", message: notice });
    }
  };

  const addFolder = async () => {
    setActionState({ kind: "working", action: "Opening the folder picker…" });
    let picked: string | null;
    try {
      picked = await platform.pickScanRootDirectory();
    } catch {
      if (mounted.current) {
        setActionState({
          kind: "error",
          message: "Fruitboard could not open the folder picker. Try again.",
        });
      }
      return;
    }
    if (picked === null) {
      if (mounted.current) {
        setActionState({ kind: "idle" });
      }
      return;
    }
    if (mounted.current) {
      setActionState({ kind: "working", action: "Adding the folder…" });
    }
    try {
      await platform.addScanRoot(defaultDisplayName(picked), picked);
    } catch (error) {
      if (mounted.current) {
        setActionState({ kind: "error", message: addFailureMessage(error) });
      }
      return;
    }
    await refreshAfterMutation("Folder added.");
  };

  const removeRoot = async (id: string) => {
    setConfirmingRemoval(null);
    setActionState({ kind: "working", action: "Removing the folder…" });
    try {
      await platform.removeScanRoot(id);
    } catch (error) {
      if (
        error instanceof PlatformError &&
        (error.code === "not_found" || error.code === "conflict")
      ) {
        await refreshAfterMutation("Folder removed.");
        return;
      }
      if (mounted.current) {
        setActionState({
          kind: "error",
          message: "Fruitboard could not remove that folder. Try again.",
        });
      }
      return;
    }
    await refreshAfterMutation("Folder removed.");
    if (mounted.current) {
      // Defer past the commit so focus lands on the live button, not a
      // node React is about to replace.
      window.setTimeout(() => {
        if (mounted.current) {
          addButtonReference.current?.focus();
        }
      }, 0);
    }
  };

  const saveRename = async (id: string, draft: string) => {
    const displayName = draft.trim();
    setActionState({ kind: "working", action: "Saving the name…" });
    try {
      await platform.updateScanRootDisplayName(id, displayName);
    } catch (error) {
      if (mounted.current) {
        // Keep the editor open with the draft so nothing typed is lost.
        setActionState({
          kind: "error",
          message:
            error instanceof PlatformError && error.code === "not_found"
              ? "That folder is no longer tracked. Reopen Preferences to confirm."
              : "Fruitboard could not save that name. Try again.",
        });
      }
      return;
    }
    if (mounted.current) {
      setRenameState({ kind: "closed" });
    }
    await refreshAfterMutation("Name saved.");
  };

  const toggleEnabled = async (root: ScanRoot) => {
    setActionState({ kind: "working", action: "Saving the setting…" });
    try {
      await platform.setScanRootEnabled(root.id, !root.enabled);
      await refresh();
      if (mounted.current) {
        setActionState({ kind: "idle" });
      }
    } catch {
      if (mounted.current) {
        setActionState({
          kind: "error",
          message: "Fruitboard could not save that setting. Try again.",
        });
      }
    }
  };

  if (loadState.kind === "loading") {
    return (
      <section
        aria-busy="true"
        aria-labelledby="scan-roots-title"
        className="preferences-card preference-state"
      >
        <h2 id="scan-roots-title">Scan roots</h2>
        <p aria-live="polite" role="status">
          Loading your folders…
        </p>
      </section>
    );
  }

  if (loadState.kind === "error") {
    return (
      <section
        aria-labelledby="scan-roots-error-title"
        className="preferences-card preference-state preference-state--error"
      >
        <div role="alert">
          <h2 id="scan-roots-error-title">Folders unavailable</h2>
          <p>Your folders were not changed.</p>
        </div>
        <button className="preference-button" onClick={retry} type="button">
          Try again
        </button>
      </section>
    );
  }

  const busy = actionState.kind === "working";

  return (
    <section aria-labelledby="scan-roots-title" className="preferences-card">
      <div className="section-heading section-heading--compact">
        <div>
          <p className="eyebrow">Local folders</p>
          <h2 id="scan-roots-title">Scan roots</h2>
        </div>
        <p>
          Choose the folders Fruitboard may look inside. Nothing is scanned yet;
          removing a folder only forgets it here and never deletes files.
        </p>
      </div>

      {loadState.roots.length === 0 ? (
        <div className="scan-roots-onboarding">
          <p className="preference-hint" role="status">
            No folders yet. Folders you add are only listed for now — scanning
            arrives in a later phase.
          </p>
          <ol className="scan-roots-steps">
            <li>Choose a folder with Add folder below.</li>
            <li>Name it anything recognizable.</li>
            <li>Turn folders off anytime without losing them.</li>
          </ol>
        </div>
      ) : (
        <ul className="scan-roots-list">
          {loadState.roots.map((root) => (
            <li className="scan-roots-item" key={root.id}>
              <div className="scan-roots-item__copy">
                {renameState.kind === "editing" &&
                renameState.id === root.id ? (
                  <form
                    className="scan-roots-rename"
                    onSubmit={(event) => {
                      event.preventDefault();
                      void saveRename(root.id, renameState.draft);
                    }}
                  >
                    <label htmlFor={`rename-${root.id}`}>Folder name</label>
                    <input
                      disabled={busy}
                      id={`rename-${root.id}`}
                      onChange={(event) => {
                        setRenameState({
                          kind: "editing",
                          id: root.id,
                          draft: event.target.value,
                        });
                      }}
                      value={renameState.draft}
                    />
                    <div className="scan-roots-item__actions">
                      <button
                        className="preference-button"
                        disabled={busy || renameState.draft.trim() === ""}
                        type="submit"
                      >
                        Save name
                      </button>
                      <button
                        className="preference-button preference-button--secondary"
                        disabled={busy}
                        onClick={() => {
                          setRenameState({ kind: "closed" });
                          setActionState({ kind: "idle" });
                        }}
                        type="button"
                      >
                        Cancel
                      </button>
                    </div>
                  </form>
                ) : (
                  <>
                    <strong>{root.displayName}</strong>
                    <span className="scan-roots-item__path">
                      {root.canonicalPath}
                    </span>
                    <span className="scan-roots-item__status">
                      {availabilityLabels[root.availability]} · Not scanned yet
                    </span>
                  </>
                )}
              </div>
              <div className="scan-roots-item__settings">
                <label className="scan-roots-toggle">
                  <input
                    checked={root.enabled}
                    disabled={busy}
                    onChange={() => void toggleEnabled(root)}
                    type="checkbox"
                  />
                  Enabled
                </label>
                {renameState.kind !== "editing" && (
                  <div className="scan-roots-item__actions">
                    <button
                      className="preference-button preference-button--secondary"
                      disabled={busy}
                      onClick={() => {
                        setRenameState({
                          kind: "editing",
                          id: root.id,
                          draft: root.displayName,
                        });
                        setActionState({ kind: "idle" });
                      }}
                      type="button"
                    >
                      Rename
                    </button>
                    {confirmingRemoval === root.id ? (
                      <>
                        <button
                          className="preference-button"
                          disabled={busy}
                          onClick={() => void removeRoot(root.id)}
                          type="button"
                        >
                          Confirm remove
                        </button>
                        <button
                          className="preference-button preference-button--secondary"
                          disabled={busy}
                          onClick={() => {
                            setConfirmingRemoval(null);
                          }}
                          type="button"
                        >
                          Keep
                        </button>
                      </>
                    ) : (
                      <button
                        className="preference-button preference-button--secondary"
                        disabled={busy}
                        onClick={() => {
                          setConfirmingRemoval(root.id);
                        }}
                        type="button"
                      >
                        Remove
                      </button>
                    )}
                  </div>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}

      <button
        aria-busy={busy}
        className="preference-button"
        disabled={busy}
        onClick={() => void addFolder()}
        ref={addButtonReference}
        type="button"
      >
        {busy && actionState.kind === "working"
          ? actionState.action
          : "Add folder"}
      </button>

      {actionState.kind === "notice" && (
        <p aria-live="polite" className="preference-message" role="status">
          {actionState.message}
        </p>
      )}
      {actionState.kind === "error" && (
        <p
          className="preference-message preference-message--error"
          role="alert"
        >
          {actionState.message}
        </p>
      )}
    </section>
  );
}
