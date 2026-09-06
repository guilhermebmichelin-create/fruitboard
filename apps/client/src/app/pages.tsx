import {
  CircleAlert,
  Columns3,
  FolderOpen,
  MonitorSmartphone,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { Link } from "react-router";
import { AppIcon } from "./AppIcon";
import { ScanRootsManager } from "./ScanRootsManager";
import { StartupViewPreference } from "./StartupViewPreference";
import type { PlatformPort } from "../platform/contracts";

export function HomePage() {
  return (
    <div className="content-stack">
      <section
        aria-labelledby="home-state-title"
        className="state-panel state-panel--initial"
        data-shell-state="initial"
      >
        <div className="state-panel__icon">
          <AppIcon icon={Sparkles} size="large" />
        </div>
        <p className="eyebrow">Foundation ready</p>
        <h2 id="home-state-title">A calm home for every project.</h2>
        <p className="state-panel__description">
          Fruitboard is establishing its local, accessible foundation before
          project discovery and production tools arrive.
        </p>
      </section>

      <section aria-labelledby="foundation-title" className="section-stack">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Current boundary</p>
            <h2 id="foundation-title">Built for focused, local work</h2>
          </div>
          <p>
            These foundations are active now. Product data and workflows remain
            deliberately out of this slice.
          </p>
        </div>

        <div className="foundation-grid">
          <article className="foundation-card">
            <AppIcon icon={ShieldCheck} />
            <h3>Local by default</h3>
            <p>
              The desktop host exposes only health, startup preference, and
              scan-root commands. Folders are listed, never scanned yet.
            </p>
          </article>
          <article className="foundation-card">
            <AppIcon icon={MonitorSmartphone} />
            <h3>One shared client</h3>
            <p>The same vocabulary can adapt to desktop and narrow screens.</p>
          </article>
        </div>
      </section>
    </div>
  );
}

export function LibraryPage() {
  return (
    <section
      aria-labelledby="library-state-title"
      className="state-panel state-panel--empty"
      data-shell-state="empty"
    >
      <div className="state-panel__icon">
        <AppIcon icon={FolderOpen} size="large" />
      </div>
      <p className="eyebrow">Empty library</p>
      <h2 id="library-state-title">No projects yet</h2>
      <p className="state-panel__description">
        Projects will appear here after safe folder discovery lands in its
        dedicated phase.
      </p>
    </section>
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
