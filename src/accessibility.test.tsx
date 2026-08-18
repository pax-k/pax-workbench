import axe from "axe-core";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import App from "./App";
import { CollaborationPanel } from "./components/CollaborationPanel";

const automatedRules = {
  // jsdom has no layout or painted pixels; contrast is covered by the CSS
  // behavior contract and deterministic browser/native visual checks.
  "color-contrast": { enabled: false },
} as const;

async function expectNoAutomatedViolations(container: HTMLElement) {
  const results = await axe.run(container, { rules: automatedRules });
  expect(
    results.violations.map(({ id, impact, nodes }) => ({
      id,
      impact,
      targets: nodes.map((node) => node.target),
    })),
  ).toEqual([]);
}

describe("critical workflow accessibility", () => {
  it("passes the automated semantic scan in the local solo shell state", async () => {
    const { container } = render(<App />);

    expect(screen.getByRole("main")).toHaveAttribute("data-workflow-mode", "localSolo");
    await expectNoAutomatedViolations(container);
  });

  it("passes the automated semantic scan with the collaboration dialog open", async () => {
    const { container } = render(
      <CollaborationPanel
        root="/tmp/accessibility"
        projectName="accessibility"
        nativeAvailable={false}
        disabled={false}
        goalRecovery={null}
        onEvent={vi.fn()}
        onRepositoryResult={vi.fn()}
        onSharedResult={vi.fn()}
        onGoalRecovery={vi.fn()}
        onBusyChange={vi.fn()}
        onProjectionChange={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /Collaboration.*Local solo/i }));
    expect(screen.getByRole("dialog", { name: "Collaboration authority" })).toHaveAttribute(
      "aria-modal",
      "true",
    );
    await expectNoAutomatedViolations(container);
  });
});
