import {
  ArrowUpRight,
  CircleAlert,
  Columns3,
  FolderOpen,
  LibraryBig,
} from "lucide-react";
import { Link } from "react-router";
import { AppIcon } from "./AppIcon";
import { ScanRootsManager } from "./ScanRootsManager";
import { StartupViewPreference } from "./StartupViewPreference";
import type { PlatformPort } from "../platform/contracts";
export { LibraryPage } from "../library/LibraryPage";

export function HomePage() {
  return (
    <div className="content-stack workspace-home">
      <section
        aria-labelledby="home-state-title"
        className="state-panel workspace-intro"
        data-shell-state="initial"
      >
        <div className="state-panel__icon">
          <AppIcon icon={LibraryBig} size="large" />
        </div>
        <h2 id="home-state-title">FL Studio projects</h2>
        <p className="state-panel__description">
          Choose a project folder in Preferences, then scan it from Library when
          scanning is available.
        </p>
        <Link className="inline-action" to="/library">
          Open Library
        </Link>
      </section>

      <section aria-label="Library workflow" className="foundation-grid">
        <article className="foundation-card workspace-module">
          <AppIcon icon={FolderOpen} />
          <h3>Project folders</h3>
          <p>Choose the local folders you want in your Library.</p>
          <Link className="workspace-module__link" to="/preferences">
            Manage folders <AppIcon icon={ArrowUpRight} size="small" />
          </Link>
        </article>
        <article className="foundation-card workspace-module workspace-module--details">
          <AppIcon icon={LibraryBig} />
          <h3>Project inspection</h3>
          <p>
            Open a project to inspect its saved details when analysis is
            available. Fruitboard leaves the project file unchanged.
          </p>
        </article>
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
