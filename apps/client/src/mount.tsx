import { createRoot } from "react-dom/client";
import { AppErrorBoundary } from "./app/AppErrorBoundary";
import { StartupRouter } from "./app/StartupRouter";
import type { PlatformPort } from "./platform/contracts";
import type {
  LibraryRenderContext,
  LibraryScanAdapter,
} from "./library/contracts";

export interface MountOptions {
  readonly libraryRenderContext?: LibraryRenderContext;
}

export function mountFruitboard(
  container: HTMLElement,
  platform: PlatformPort,
  libraryAdapter?: LibraryScanAdapter,
  options: MountOptions = {},
) {
  // React's default root callbacks print Error objects. Native logging owns
  // diagnostics, so renderer failures must not copy paths or tokens to console.
  const containUntrustedRendererDiagnostic = () => undefined;
  const root = createRoot(container, {
    onCaughtError: containUntrustedRendererDiagnostic,
    onRecoverableError: containUntrustedRendererDiagnostic,
    onUncaughtError: containUntrustedRendererDiagnostic,
  });

  root.render(
    <AppErrorBoundary>
      <StartupRouter
        libraryAdapter={libraryAdapter}
        libraryRenderContext={options.libraryRenderContext ?? "native"}
        onError={containUntrustedRendererDiagnostic}
        platform={platform}
      />
    </AppErrorBoundary>,
  );
}
