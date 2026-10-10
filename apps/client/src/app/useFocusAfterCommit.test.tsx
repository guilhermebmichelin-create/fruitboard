import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { useFocusAfterCommit } from "./useFocusAfterCommit";

function FocusHarness({
  visible = false,
  disabled = false,
  isCurrent = () => true,
  onPrepare,
}: {
  readonly visible?: boolean;
  readonly disabled?: boolean;
  readonly isCurrent?: () => boolean;
  readonly onPrepare?: (complete: () => void) => void;
}) {
  const target = useRef<HTMLButtonElement | null>(null);
  const { requestFocus, prepareFocus, cancelPendingFocus } =
    useFocusAfterCommit();
  return (
    <>
      <button onClick={() => requestFocus(() => target.current, isCurrent)}>
        Request focus
      </button>
      <button
        onClick={() => {
          const restoreFocus = prepareFocus();
          onPrepare?.(() => restoreFocus(() => target.current, isCurrent));
        }}
      >
        Prepare action
      </button>
      <button onClick={cancelPendingFocus}>Cancel focus</button>
      <button>Elsewhere</button>
      {visible && (
        <button disabled={disabled} ref={target}>
          Target
        </button>
      )}
    </>
  );
}

describe("useFocusAfterCommit", () => {
  it("waits through delayed commits and disabled targets until focus is possible", async () => {
    const view = render(<FocusHarness />);
    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "Request focus" }));
    for (let commit = 0; commit < 12; commit++) {
      view.rerender(<FocusHarness visible disabled />);
    }
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Request focus" }),
    );
    view.rerender(<FocusHarness visible />);
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Target" }),
    );
  });

  it("drops a pending request when its action becomes stale", async () => {
    let current = true;
    const isCurrent = () => current;
    const view = render(<FocusHarness isCurrent={isCurrent} />);
    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "Request focus" }));
    current = false;
    view.rerender(<FocusHarness visible isCurrent={isCurrent} />);
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Request focus" }),
    );
  });

  it("retains the request when removing a control returns focus to the document", async () => {
    const view = render(<FocusHarness />);
    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "Request focus" }));
    fireEvent.focusIn(document.body);
    view.rerender(<FocusHarness visible />);
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Target" }),
    );
  });

  it.each(["focus", "pointer", "keyboard", "cancel"] as const)(
    "preserves later user intent after %s interaction",
    async (interaction) => {
      const view = render(<FocusHarness />);
      await userEvent
        .setup()
        .click(screen.getByRole("button", { name: "Request focus" }));
      const elsewhere = screen.getByRole("button", { name: "Elsewhere" });
      if (interaction === "focus") elsewhere.focus();
      if (interaction === "pointer") fireEvent.pointerDown(elsewhere);
      if (interaction === "keyboard")
        fireEvent.keyDown(document, { key: "Tab" });
      if (interaction === "cancel")
        fireEvent.click(screen.getByRole("button", { name: "Cancel focus" }));
      const activeElement = document.activeElement;
      view.rerender(<FocusHarness visible />);
      expect(document.activeElement).toBe(activeElement);
    },
  );

  it("ignores a slow completion after a newer action prepares focus", () => {
    const completions: (() => void)[] = [];
    render(
      <FocusHarness
        visible
        onPrepare={(complete) => completions.push(complete)}
      />,
    );
    const prepare = screen.getByRole("button", { name: "Prepare action" });
    fireEvent.click(prepare);
    fireEvent.click(prepare);
    const target = screen.getByRole("button", { name: "Target" });
    const focus = vi.spyOn(target, "focus");
    act(() => completions[0]?.());
    expect(focus).not.toHaveBeenCalled();
    act(() => completions[1]?.());
    expect(document.activeElement).toBe(target);
  });

  it("ignores prepared work after unmount", () => {
    let complete: (() => void) | undefined;
    const view = render(
      <FocusHarness
        visible
        onPrepare={(next) => {
          complete = next;
        }}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Prepare action" }));
    const target = screen.getByRole("button", { name: "Target" });
    const focus = vi.spyOn(target, "focus");
    view.unmount();
    act(() => complete?.());
    expect(focus).not.toHaveBeenCalled();
  });
});
