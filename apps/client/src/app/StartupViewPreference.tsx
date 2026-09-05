import {
  useEffect,
  useRef,
  useState,
  type ChangeEvent,
  type FormEvent,
} from "react";
import {
  STARTUP_VIEWS,
  type PlatformPort,
  type StartupView,
} from "../platform/contracts";

const labels: Readonly<Record<StartupView, string>> = {
  home: "Home",
  library: "Library",
  board: "Board",
  preferences: "Preferences",
};

type LoadState =
  | { readonly kind: "loading" }
  | { readonly kind: "error" }
  | {
      readonly kind: "ready";
      readonly saved: StartupView;
      readonly draft: StartupView;
    };

type SaveState = "idle" | "saving" | "saved" | "error";

export function StartupViewPreferenceControl({
  platform,
}: {
  readonly platform: PlatformPort;
}) {
  const [loadAttempt, setLoadAttempt] = useState(0);
  const [loadState, setLoadState] = useState<LoadState>({ kind: "loading" });
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  useEffect(() => {
    let current = true;
    void platform.getStartupView().then(
      ({ startupView }) => {
        if (current) {
          setLoadState({
            kind: "ready",
            saved: startupView,
            draft: startupView,
          });
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

  const selectView = (event: ChangeEvent<HTMLSelectElement>) => {
    if (loadState.kind !== "ready") {
      return;
    }
    setLoadState({
      ...loadState,
      draft: event.target.value as StartupView,
    });
    setSaveState("idle");
  };

  const save = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (loadState.kind !== "ready" || saveState === "saving") {
      return;
    }

    setSaveState("saving");
    try {
      const { startupView } = await platform.setStartupView(loadState.draft);
      if (mounted.current) {
        setLoadState({
          kind: "ready",
          saved: startupView,
          draft: startupView,
        });
        setSaveState("saved");
      }
    } catch {
      if (mounted.current) {
        setSaveState("error");
      }
    }
  };

  if (loadState.kind === "loading") {
    return (
      <section
        aria-busy="true"
        aria-labelledby="startup-view-title"
        className="preferences-card preference-state"
      >
        <h2 id="startup-view-title">Startup view</h2>
        <p aria-live="polite" role="status">
          Loading your preference…
        </p>
      </section>
    );
  }

  if (loadState.kind === "error") {
    return (
      <section
        aria-labelledby="startup-view-error-title"
        className="preferences-card preference-state preference-state--error"
      >
        <div role="alert">
          <h2 id="startup-view-error-title">Startup preference unavailable</h2>
          <p>Your existing preference was not changed.</p>
        </div>
        <button
          className="preference-button"
          onClick={() => {
            setLoadState({ kind: "loading" });
            setSaveState("idle");
            setLoadAttempt((attempt) => attempt + 1);
          }}
          type="button"
        >
          Try again
        </button>
      </section>
    );
  }

  const changed = loadState.draft !== loadState.saved;

  return (
    <section aria-labelledby="startup-view-title" className="preferences-card">
      <div className="section-heading section-heading--compact">
        <div>
          <p className="eyebrow">Local preference</p>
          <h2 id="startup-view-title">Startup view</h2>
        </div>
        <p>Choose the page Fruitboard opens when the desktop app starts.</p>
      </div>

      <form
        aria-busy={saveState === "saving"}
        className="preference-form"
        onSubmit={(event) => void save(event)}
      >
        <div className="preference-field">
          <label htmlFor="startup-view">Open Fruitboard on</label>
          <select
            disabled={saveState === "saving"}
            id="startup-view"
            onChange={selectView}
            value={loadState.draft}
          >
            {STARTUP_VIEWS.map((startupView) => (
              <option key={startupView} value={startupView}>
                {labels[startupView]}
              </option>
            ))}
          </select>
          <p className="preference-hint">
            {loadState.saved === "home"
              ? "Home is the default startup view."
              : `Fruitboard currently starts on ${labels[loadState.saved]}.`}
          </p>
        </div>

        <button
          className="preference-button"
          disabled={!changed || saveState === "saving"}
          type="submit"
        >
          {saveState === "saving" ? "Saving…" : "Save preference"}
        </button>
      </form>

      {saveState === "saved" && (
        <p aria-live="polite" className="preference-message" role="status">
          Startup view saved.
        </p>
      )}
      {saveState === "error" && (
        <p
          className="preference-message preference-message--error"
          role="alert"
        >
          Fruitboard could not save this preference. Try again.
        </p>
      )}
    </section>
  );
}
