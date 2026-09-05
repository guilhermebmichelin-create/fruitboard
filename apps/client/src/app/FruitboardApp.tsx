import { CircleAlert, LoaderCircle } from "lucide-react";
import { useEffect, useRef, useState, type MouseEvent } from "react";
import { Link, NavLink, Outlet, useLocation } from "react-router";
import type { AppHealth, PlatformPort } from "../platform/contracts";
import { AppIcon } from "./AppIcon";
import { getNavigationItem, navigationItems } from "./navigation";

interface FruitboardAppProps {
  readonly platform: PlatformPort;
}

type HealthState =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly health: AppHealth }
  | { readonly kind: "error" };

function NativeConnection({ state }: { readonly state: HealthState }) {
  return (
    <aside aria-labelledby="connection-title" className="connection-panel">
      <p className="connection-panel__label" id="connection-title">
        Desktop connection
      </p>

      {state.kind === "loading" && (
        <div
          aria-label="Desktop connection status"
          aria-live="polite"
          className="connection-status"
          data-shell-state="loading"
          role="status"
        >
          <AppIcon icon={LoaderCircle} size="small" />
          <span>Connecting…</span>
        </div>
      )}

      {state.kind === "ready" && (
        <div
          aria-label="Desktop connection status"
          aria-live="polite"
          className="connection-status"
          role="status"
        >
          <span aria-hidden="true" className="connection-status__dot" />
          <span>Connected</span>
          <span className="connection-status__version">
            v{state.health.version}
          </span>
        </div>
      )}

      {state.kind === "error" && (
        <div
          aria-label="Desktop connection error"
          className="connection-status connection-status--error"
          data-shell-state="error"
          role="alert"
        >
          <AppIcon icon={CircleAlert} size="small" />
          <span>Connection unavailable</span>
        </div>
      )}
    </aside>
  );
}

function RouteFocusManager() {
  const location = useLocation();
  const previousPath = useRef(location.pathname);

  useEffect(() => {
    if (previousPath.current !== location.pathname) {
      document.querySelector<HTMLElement>("#main-content")?.focus();
      previousPath.current = location.pathname;
    }
  }, [location.pathname]);

  return null;
}

export function FruitboardApp({ platform }: FruitboardAppProps) {
  const location = useLocation();
  const currentRoute = getNavigationItem(location.pathname);
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

  useEffect(() => {
    document.title = `${currentRoute.title} · Fruitboard`;
  }, [currentRoute.title]);

  const focusMainContent = (event: MouseEvent<HTMLAnchorElement>) => {
    event.preventDefault();
    document.querySelector<HTMLElement>("#main-content")?.focus();
  };

  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content" onClick={focusMainContent}>
        Skip to main content
      </a>

      <div className="brand-block">
        <span aria-hidden="true" className="brand-mark">
          F
        </span>
        <span className="brand-copy">
          <strong>Fruitboard</strong>
          <span>Local workspace</span>
        </span>
      </div>

      <nav aria-label="Primary" className="primary-navigation">
        <ul className="primary-navigation__list">
          {navigationItems.map((item) => (
            <li key={item.path}>
              <NavLink
                className={({ isActive }) =>
                  `navigation-link${isActive ? " navigation-link--active" : ""}`
                }
                end={item.path === "/"}
                to={item.path}
              >
                <AppIcon icon={item.icon} />
                <span>{item.label}</span>
              </NavLink>
            </li>
          ))}
        </ul>
      </nav>

      <NativeConnection state={healthState} />

      <header className="page-header">
        <div className="page-header__copy">
          <p className="eyebrow">Fruitboard workspace</p>
          <h1>{currentRoute.title}</h1>
          <p>{currentRoute.description}</p>
        </div>
        <div aria-label="Page actions" className="page-actions">
          <Link className="header-action" to={currentRoute.actionPath}>
            {currentRoute.actionLabel}
          </Link>
        </div>
      </header>

      <main className="main-content" id="main-content" tabIndex={-1}>
        <RouteFocusManager />
        <Outlet />
      </main>
    </div>
  );
}
