import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");

describe("deterministic accessibility and visual behavior contract", () => {
  it("pins readable type floors and visible keyboard focus", () => {
    expect(css).toContain("body { font-size: 13px; }");
    expect(css).toContain("font-size: max(13px, 1em) !important");
    expect(css).toContain("font-size: max(11px, 1em) !important");
    expect(css).toMatch(/:focus-visible[^}]*outline:\s*3px solid var\(--blueprint\)/u);
  });

  it("pins zoom, increased-contrast, forced-color, and reduced-motion adaptations", () => {
    expect({
      compactZoomLayout: css.includes("@media (max-width: 700px)"),
      increasedContrast: css.includes("@media (prefers-contrast: more)"),
      forcedColors: css.includes("@media (forced-colors: active)"),
      reducedMotion: css.includes("@media (prefers-reduced-motion: reduce)"),
      navigationOverlay: css.includes("width: min(320px, 88vw)"),
      inspectorOverlay: css.includes("width: min(340px, 88vw)"),
    }).toMatchInlineSnapshot(`
      {
        "compactZoomLayout": true,
        "forcedColors": true,
        "increasedContrast": true,
        "inspectorOverlay": true,
        "navigationOverlay": true,
        "reducedMotion": true,
      }
    `);
  });
});
