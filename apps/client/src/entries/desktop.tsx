import "../styles.css";
import { mountFruitboard } from "../mount";
import { createTauriPlatform } from "../platform/tauri";

const container = document.querySelector<HTMLElement>("#root");

if (!container) {
  throw new Error("The Fruitboard root element is missing.");
}

mountFruitboard(container, createTauriPlatform());
