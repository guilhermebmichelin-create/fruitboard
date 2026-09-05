import { StrictMode, useEffect, useState } from "react";
import { RouterProvider } from "react-router/dom";
import type { PlatformPort, StartupView } from "../platform/contracts";
import { createFruitboardHashRouter } from "./router";

const startupPaths: Readonly<Record<StartupView, string>> = {
  home: "/",
  library: "/library",
  board: "/board",
  preferences: "/preferences",
};

type StartupState =
  | { readonly kind: "loading" }
  | { readonly kind: "error" }
  | { readonly kind: "ready"; readonly startupView: StartupView | null };

const hasExplicitRoute = () =>
  window.location.hash !== "" && window.location.hash !== "#";

function ReadyFruitboard({
  onError,
  platform,
  startupView,
}: {
  readonly onError: () => undefined;
  readonly platform: PlatformPort;
  readonly startupView: StartupView | null;
}) {
  const [router] = useState(() => createFruitboardHashRouter(platform));

  useEffect(() => {
    if (startupView !== null && !hasExplicitRoute()) {
      void router.navigate(startupPaths[startupView], { replace: true });
    }
  }, [router, startupView]);

  return (
    <StrictMode>
      <RouterProvider onError={onError} router={router} />
    </StrictMode>
  );
}

export function StartupRouter({
  onError,
  platform,
}: {
  readonly onError: () => undefined;
  readonly platform: PlatformPort;
}) {
  const [loadPreference] = useState(() => !hasExplicitRoute());
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<StartupState>(() =>
    loadPreference ? { kind: "loading" } : { kind: "ready", startupView: null },
  );

  useEffect(() => {
    if (!loadPreference) {
      return;
    }

    let current = true;
    void platform.getStartupView().then(
      ({ startupView }) => {
        if (current) {
          setState({ kind: "ready", startupView });
        }
      },
      () => {
        if (current) {
          setState({ kind: "error" });
        }
      },
    );

    return () => {
      current = false;
    };
  }, [attempt, loadPreference, platform]);

  if (state.kind === "ready") {
    return (
      <ReadyFruitboard
        onError={onError}
        platform={platform}
        startupView={state.startupView}
      />
    );
  }

  if (state.kind === "error") {
    return (
      <main className="startup-gate">
        <section
          aria-labelledby="startup-error-title"
          className="startup-gate__panel startup-gate__panel--error"
        >
          <div role="alert">
            <h1 id="startup-error-title">Startup preference unavailable</h1>
            <p>
              Fruitboard could not read your preferred page. Your saved data was
              not changed.
            </p>
          </div>
          <div className="startup-gate__actions">
            <button
              className="preference-button"
              onClick={() => {
                setState({ kind: "loading" });
                setAttempt((value) => value + 1);
              }}
              type="button"
            >
              Try again
            </button>
            <button
              className="startup-gate__secondary"
              onClick={() => setState({ kind: "ready", startupView: "home" })}
              type="button"
            >
              Open Home
            </button>
          </div>
        </section>
      </main>
    );
  }

  return (
    <main aria-busy="true" className="startup-gate">
      <section
        aria-labelledby="startup-loading-title"
        className="startup-gate__panel"
      >
        <p className="eyebrow">Local preference</p>
        <h1 id="startup-loading-title">Opening Fruitboard</h1>
        <p aria-live="polite" role="status">
          Loading your startup view…
        </p>
      </section>
    </main>
  );
}
