import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");
const tauriConfig = JSON.parse(
  readFileSync(resolve(process.cwd(), "src-tauri/tauri.conf.json"), "utf8"),
) as {
  app: { windows: Array<{ minWidth: number; minHeight: number; resizable: boolean }> };
};

describe("responsive application shell contract", () => {
  it("permits the signed native window to reach the founder acceptance viewport", () => {
    expect(tauriConfig.app.windows[0]).toMatchObject({
      minWidth: 900,
      minHeight: 700,
      resizable: true,
    });
  });

  it("progressively discloses secondary panes without imposing a page-width floor", () => {
    expect(css).toContain("@media (max-width: 1100px)");
    expect(css).toContain('[data-navigation-open="false"]');
    expect(css).toContain('[data-inspector-open="false"]');
    expect(css).toMatch(/body\s*\{[^}]*overflow:\s*hidden/u);
    expect(css).not.toContain("min-width: 1080px");
  });
});
