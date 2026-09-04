import { useEffect, useState } from "react";
import type { AppHealth, PlatformPort } from "../platform/contracts";

interface FruitboardAppProps {
  readonly platform: PlatformPort;
}

type HealthState =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly health: AppHealth }
  | { readonly kind: "error" };

export function FruitboardApp({ platform }: FruitboardAppProps) {
  const [healthState, setHealthState] = useState<HealthState>({
    kind: "loading",
  });

  useEffect(() => {
    let active = true;

    void platform.getAppHealth().then(
      (health) => {
        if (active) {
          setHealthState({ kind: "ready", health });
        }
      },
      () => {
        if (active) {
          setHealthState({ kind: "error" });
        }
      },
    );

    return () => {
      active = false;
    };
  }, [platform]);

  return (
    <main>
      <p>Desktop foundation</p>
      <h1>Fruitboard</h1>
      <p>A local-first FL Studio project library.</p>

      {healthState.kind === "loading" && (
        <p role="status">Checking the native connection…</p>
      )}

      {healthState.kind === "ready" && (
        <dl aria-label="Native application status">
          <div>
            <dt>Native status</dt>
            <dd>Connected</dd>
          </div>
          <div>
            <dt>Version</dt>
            <dd>{healthState.health.version}</dd>
          </div>
        </dl>
      )}

      {healthState.kind === "error" && (
        <p role="alert">
          Fruitboard could not reach its native application host.
        </p>
      )}
    </main>
  );
}
