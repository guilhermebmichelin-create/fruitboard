import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";
import "./canvas";

afterEach(() => {
  cleanup();
});
