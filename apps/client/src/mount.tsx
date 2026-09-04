import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { FruitboardApp } from "./app/FruitboardApp";
import type { PlatformPort } from "./platform/contracts";

export function mountFruitboard(
  container: HTMLElement,
  platform: PlatformPort,
) {
  createRoot(container).render(
    <StrictMode>
      <FruitboardApp platform={platform} />
    </StrictMode>,
  );
}
