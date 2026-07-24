import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";

const configSource = readFileSync(
  new URL("../rspress.config.ts", import.meta.url),
  "utf8",
);
const switchSource = readFileSync(
  new URL("./DocsSwitchAppearance.tsx", import.meta.url),
  "utf8",
);
const navTitleSource = readFileSync(
  new URL("./DocsNavTitle.tsx", import.meta.url),
  "utf8",
);

test("rspress routes internal theme imports through the local theme entry", () => {
  expect(configSource).toContain("builderConfig");
  expect(configSource).toContain("'@rspress/core/theme'");
  expect(configSource).toContain("path.join(__dirname, 'theme/index.tsx')");
});

test("local theme helpers use theme-original exports to avoid alias recursion", () => {
  expect(switchSource).toContain('@rspress/core/theme-original');
  expect(navTitleSource).toContain('@rspress/core/theme-original');
});
