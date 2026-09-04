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
            <p>The desktop host exposes one narrow health command.</p>
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
        Projects will appear here after storage and safe folder discovery land
        in their dedicated phases.
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

export function PreferencesPage() {
  return (
    <section aria-labelledby="preferences-title" className="preferences-card">
      <div className="section-heading section-heading--compact">
        <div>
          <p className="eyebrow">Interface baseline</p>
          <h2 id="preferences-title">Designed to adapt without surprises</h2>
        </div>
      </div>
      <dl className="decision-list">
        <div>
          <dt>Appearance</dt>
          <dd>Light foundation; dark mode is deferred, not rejected.</dd>
        </div>
        <div>
          <dt>Motion</dt>
          <dd>Reduced-motion preferences are respected automatically.</dd>
        </div>
        <div>
          <dt>Narrow layout</dt>
          <dd>Touch-sized navigation without PWA or scanner behavior.</dd>
        </div>
      </dl>
    </section>
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
