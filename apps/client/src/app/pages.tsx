import { CircleAlert, Columns3, LibraryBig, ShieldCheck } from "lucide-react";
import { Link } from "react-router";
import { AppIcon } from "./AppIcon";
import { ScanRootsManager } from "./ScanRootsManager";
import { StartupViewPreference } from "./StartupViewPreference";
import type { PlatformPort } from "../platform/contracts";
export { LibraryPage } from "../library/LibraryPage";

export function HomePage() {
  return (
    <div className="content-stack">
      <section
        aria-labelledby="home-state-title"
        className="state-panel state-panel--initial"
        data-shell-state="initial"
      >
        <div className="state-panel__icon">
          <AppIcon icon={LibraryBig} size="large" />
        </div>
        <h2 id="home-state-title">Your FL Studio project library</h2>
        <p className="state-panel__description">
          Choose your project folders in Preferences. When scanning is enabled
          in your build, open Library to discover projects in those folders.
        </p>
      </section>

      <section aria-labelledby="foundation-title" className="section-stack">
        <div className="section-heading">
          <div>
            <h2 id="foundation-title">How Library works</h2>
          </div>
        </div>

        <div className="foundation-grid">
          <article className="foundation-card">
            <AppIcon icon={ShieldCheck} />
            <h3>Local project files</h3>
            <p>
              Fruitboard keeps its library data on this device and leaves your
              FL Studio project files unchanged.
            </p>
          </article>
          <article className="foundation-card">
            <AppIcon icon={LibraryBig} />
            <h3>Saved project details</h3>
            <p>
              When analysis is enabled in your build, select a project and
              choose Analyze project to inspect supported saved details.
            </p>
          </article>
        </div>
      </section>
    </div>
  );
}

export function BoardPage() {
  return (
    <section
      aria-labelledby="board-state-title"
      className="state-panel state-panel--initial"
      data-shell-state="initial"
    >
      <div className="state-panel__icon">
        <AppIcon icon={Columns3} size="large" />
      </div>
      <p className="eyebrow">Structure only</p>
      <h2 id="board-state-title">The board is ready for future workflows</h2>
      <p className="state-panel__description">
        Columns, project cards, and production states remain deferred until the
        underlying project model is ready.
      </p>
    </section>
  );
}

export function PreferencesPage({
  platform,
}: {
  readonly platform: PlatformPort;
}) {
  return (
    <div className="content-stack">
      <StartupViewPreference platform={platform} />
      <ScanRootsManager platform={platform} />
    </div>
  );
}

export function NotFoundPage() {
  return (
    <section
      aria-labelledby="error-state-title"
      className="state-panel state-panel--error"
      data-shell-state="error"
    >
      <div className="state-panel__icon">
        <AppIcon icon={CircleAlert} size="large" />
      </div>
      <p className="eyebrow">Navigation error</p>
      <h2 id="error-state-title">That page is not available</h2>
      <p className="state-panel__description">
        The address does not match a route in this foundation shell.
      </p>
      <Link className="inline-action" to="/">
        Return home
      </Link>
    </section>
  );
}
