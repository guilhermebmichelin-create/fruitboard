import { createCanvas, type Canvas } from "@napi-rs/canvas";

// axe 4.14 compares actual glyph pixels to distinguish labels from icon-font
// ligatures. jsdom supplies no canvas renderer; use real rasterization in tests
// so missing canvas support cannot turn Label in Name checks into incomplete
// results. This module is imported only by the test setup, never the app.
const canvases = new WeakMap<HTMLCanvasElement, Canvas>();

Object.defineProperty(HTMLCanvasElement.prototype, "getContext", {
  configurable: true,
  value(this: HTMLCanvasElement, kind: string) {
    if (kind !== "2d") return null;
    let canvas = canvases.get(this);
    if (!canvas) {
      canvas = createCanvas(this.width, this.height);
      canvases.set(this, canvas);
    }
    return canvas.getContext("2d");
  },
});
