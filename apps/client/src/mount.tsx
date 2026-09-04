import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider } from "react-router/dom";
import { createFruitboardHashRouter } from "./app/router";
import type { PlatformPort } from "./platform/contracts";

export function mountFruitboard(
  container: HTMLElement,
  platform: PlatformPort,
) {
  const router = createFruitboardHashRouter(platform);

  createRoot(container).render(
    <StrictMode>
      <RouterProvider router={router} />
    </StrictMode>,
  );
}
