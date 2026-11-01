import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { describe, expect, it } from "vitest";
import { SavedNoteSequence } from "./SavedNoteSequence";
import { ProjectPatterns } from "./SavedPatterns";
import { sequenceDetails } from "./noteSequences.fixture";

describe("saved note sequence view", () => {
  it("groups by validated channel labels and preserves record ordinals and duplicates", async () => {
    const details = sequenceDetails(3);
    const user = userEvent.setup();
    const view = render(
      <ProjectPatterns
        patterns={details.patterns}
        notes={details.patternNoteCounts}
        sequences={details.patternNoteSequences}
        channels={details.channels}
      />,
    );
    const toggle = screen.getByText("Saved note sequence for pattern 1");
    toggle.focus();
    await user.keyboard("{Enter}");
    const first = screen.getByRole("region", {
      name: "Pattern 1, channel 1 note records",
    });
    const second = screen.getByRole("region", {
      name: "Pattern 1, channel 2 note records",
    });
    expect(within(first).getByText("NoteOne")).toBeTruthy();
    expect(within(second).getByText("NoteTwo")).toBeTruthy();
    expect(within(first).getAllByRole("listitem")).toHaveLength(2);
    expect(within(first).getByText("Saved record 3")).toBeTruthy();
    expect(within(second).getByText("Saved record 2")).toBeTruthy();
    expect(within(first).getAllByText("60")).toHaveLength(2);
    expect(
      screen.getByText(/Key and velocity are raw saved numbers/),
    ).toBeTruthy();
    expect(view.container.querySelectorAll("audio, canvas, a")).toHaveLength(0);
    expect(
      (
        await axe.run(view.container, {
          rules: { "color-contrast": { enabled: false } },
        })
      ).violations,
    ).toEqual([]);
  });
  it("reveals all records in batches of 64 and retains keyboard focus", async () => {
    const details = sequenceDetails(130);
    const user = userEvent.setup();
    render(
      <SavedNoteSequence
        patternId={1}
        channels={details.channels}
        sequence={details.patternNoteSequences.items[0]!.sequence}
      />,
    );
    await user.click(screen.getByText("Saved note sequence for pattern 1"));
    expect(screen.getAllByRole("listitem")).toHaveLength(64);
    const control = screen.getByRole("button", {
      name: "Show more notes for pattern 1",
    });
    control.focus();
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(128);
    expect(document.activeElement).toBe(control);
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(130);
    expect(screen.getByText("Saved record 130")).toBeTruthy();
    expect(control.textContent).toBe("Show fewer notes");
    await user.keyboard("{Enter}");
    expect(screen.getAllByRole("listitem")).toHaveLength(64);
    expect(document.activeElement).toBe(control);
  });
  it("explains absent, unsupported and limit states without a false empty sequence", async () => {
    const user = userEvent.setup();
    const view = render(<SavedNoteSequence patternId={1} channels={[]} />);
    await user.click(screen.getByText("Saved note sequence for pattern 1"));
    expect(screen.getByText(/were not saved with this result/)).toBeTruthy();
    view.rerender(
      <SavedNoteSequence
        patternId={1}
        channels={[]}
        sequence={{ status: "unavailable", reason: "no_stored_notes" }}
      />,
    );
    expect(screen.getByText(/does not prove it has zero notes/)).toBeTruthy();
    view.rerender(
      <SavedNoteSequence
        patternId={1}
        channels={[]}
        sequence={{ status: "unsupported", reason: "unqualified_record" }}
      />,
    );
    expect(screen.getByText(/complete sequence is unavailable/)).toBeTruthy();
    view.rerender(
      <SavedNoteSequence patternId={1} channels={[]} reason="limit_exceeded" />,
    );
    expect(screen.getByText(/No shortened sequence is shown/)).toBeTruthy();
    expect(screen.queryByRole("list")).toBeNull();
  });
  it("renders hostile channel labels as escaped inert text", async () => {
    const details = sequenceDetails();
    const channel = details.channels[0]!;
    const user = userEvent.setup();
    const view = render(
      <SavedNoteSequence
        patternId={1}
        channels={[
          {
            ...channel,
            name: { ...channel.name, value: "<img src=x>\u202e\n" },
          },
        ]}
        sequence={details.patternNoteSequences.items[0]!.sequence}
      />,
    );
    await user.click(screen.getByText("Saved note sequence for pattern 1"));
    expect(screen.getByText("<img src=x>\\u202e\\u000a")).toBeTruthy();
    expect(view.container.querySelector("img")).toBeNull();
  });
});
