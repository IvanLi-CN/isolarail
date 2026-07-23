import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";

test("docs theme switch keeps a single active icon and a 24px centered hit area", () => {
  const css = readFileSync(
    new URL("../styles/global.css", import.meta.url),
    "utf8",
  );

  expect(css).toContain(".docs-switch-appearance");
  expect(css).toContain("width: 24px;");
  expect(css).toContain("height: 24px;");
  expect(css).toContain("display: flex;");
  expect(css).toContain("justify-content: center;");
  expect(css).toContain("align-items: center;");
  expect(css).toContain(".rp-switch-appearance__icon--sun");
  expect(css).toContain(
    ".rp-switch-appearance__icon--moon,\nhtml.rp-dark .rp-switch-appearance__icon--sun",
  );
  expect(css).toContain("html.rp-dark .rp-switch-appearance__icon--moon");
});
