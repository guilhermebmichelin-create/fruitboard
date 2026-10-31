import assert from "node:assert/strict";
import { createRequire } from "node:module";
import test from "node:test";

const rootRequire = createRequire(new URL("../package.json", import.meta.url));
const clientRequire = createRequire(
  new URL("../apps/client/package.json", import.meta.url),
);
const eslintRequire = createRequire(clientRequire.resolve("eslint"));
const typedLintRequire = createRequire(
  clientRequire.resolve("typescript-eslint"),
);
const parserRequire = createRequire(
  typedLintRequire.resolve("@typescript-eslint/parser"),
);
const consumers = [
  [
    "Markdown lint",
    createRequire(rootRequire.resolve("markdownlint-cli/package.json")),
  ],
  ["ESLint", eslintRequire],
  [
    "ESLint config",
    createRequire(eslintRequire.resolve("@eslint/config-array")),
  ],
  [
    "TypeScript parser",
    createRequire(
      parserRequire.resolve("@typescript-eslint/typescript-estree"),
    ),
  ],
];

for (const [name, consumerRequire] of consumers) {
  test(`${name} bounds glob expansion and preserves file selection`, () => {
    const { Minimatch } = consumerRequire("minimatch");
    // Eight split points reproduce the amplification without a large fixture.
    // The configured limit must also cover optimization-created patterns.
    const pattern = Array(8).fill("**/../segment/leaf").join("/");
    const bounded = new Minimatch(pattern, {
      optimizationLevel: 2,
      braceExpandMax: 8,
    });
    assert.ok(bounded.set.length > 0 && bounded.set.length <= 8);

    const sources = new Minimatch("src/**/*.{ts,tsx}", { dot: true });
    assert.equal(sources.match("src/library/ProjectDetailsPanel.tsx"), true);
    assert.equal(sources.match("src/platform/contracts.ts"), true);
    assert.equal(sources.match("src/library", true), true);
    assert.equal(sources.match("src/library/notes.md"), false);
    assert.equal(sources.match("dist/main.js"), false);
    const dependencies = new Minimatch("**/node_modules/**", { dot: true });
    assert.equal(
      dependencies.match("apps/client/node_modules/tool/index.js"),
      true,
    );
    assert.equal(dependencies.match("apps/client/src/app/router.tsx"), false);
  });
}
