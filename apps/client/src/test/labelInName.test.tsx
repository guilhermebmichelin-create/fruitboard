import { render } from "@testing-library/react";
import axe from "axe-core";
import { expect, it } from "vitest";

it.each([
  ["Rename Projects", "Rename", false],
  ["Project details Example.flp", "Hide project details", true],
  ["Hide project details Example.flp", "Hide project details", false],
])("checks the visible label in %s", async (name, label, fails) => {
  const view = render(<button aria-label={name}>{label}</button>);
  const result = await axe.run(view.container, {
    runOnly: { type: "rule", values: ["label-content-name-mismatch"] },
  });
  expect(result.incomplete).toEqual([]);
  expect(result.violations.map(({ id }) => id)).toEqual(
    fails ? ["label-content-name-mismatch"] : [],
  );
});
