import { useCallback, useEffect, useRef, useState } from "react";
import {
  PlatformError,
  type PlatformPort,
  type ScanRoot,
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

const defaultDisplayName = (path: string): string => {
  const segments = path.split(/[\\/]/).filter((segment) => segment !== "");
  const last = segments[segments.length - 1];
  return last === undefined || last === "" ? "Scan root" : last;
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
  const mounted = useRef(true);

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

  const retry = () => {
    setActionState({ kind: "idle" });
    setConfirmingRemoval(null);
    setLoadState({ kind: "loading" });
    setLoadAttempt((attempt) => attempt + 1);
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
        <p className="preference-hint" role="status">
          No folders yet. Add one to get started.
        </p>
      ) : (
        <ul className="scan-roots-list">
          {loadState.roots.map((root) => (
            <li className="scan-roots-item" key={root.id}>
              <div className="scan-roots-item__copy">
                <strong>{root.displayName}</strong>
                <span className="scan-roots-item__path">
                  {root.canonicalPath}
                </span>
              </div>
              {confirmingRemoval === root.id ? (
                <div className="scan-roots-item__actions">
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
                </div>
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
            </li>
          ))}
        </ul>
      )}

      <button
        aria-busy={busy}
        className="preference-button"
        disabled={busy}
        onClick={() => void addFolder()}
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
