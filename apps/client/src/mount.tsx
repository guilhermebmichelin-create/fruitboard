import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider } from "react-router/dom";
import { AppErrorBoundary } from "./app/AppErrorBoundary";
import { createFruitboardHashRouter } from "./app/router";
import type { PlatformPort } from "./platform/contracts";

export function mountFruitboard(
  container: HTMLElement,
  platform: PlatformPort,
) {
  const router = createFruitboardHashRouter(platform);

  // React's default root callbacks print Error objects. Native logging owns
  // diagnostics, so renderer failures must not copy paths or tokens to console.
  const containUntrustedRendererDiagnostic = () => undefined;
  const root = createRoot(container, {
    onCaughtError: containUntrustedRendererDiagnostic,
    onRecoverableError: containUntrustedRendererDiagnostic,
    onUncaughtError: containUntrustedRendererDiagnostic,
  });

  root.render(
    <StrictMode>
      <AppErrorBoundary>
        <RouterProvider router={router} />
      </AppErrorBoundary>
    </StrictMode>,
  );
}
