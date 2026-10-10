import { useCallback, useLayoutEffect, useReducer, useRef } from "react";

type FocusTarget = () => HTMLElement | null;
type FocusRequest = (target: FocusTarget, isCurrent?: () => boolean) => void;

/** Wait for React to commit an enabled target, without guessing its timing. */
export function useFocusAfterCommit() {
  const pending = useRef<{
    target: FocusTarget;
    isCurrent: () => boolean;
  } | null>(null);
  const requestVersion = useRef(0);
  const mounted = useRef(false);
  const [, scheduleCommit] = useReducer((version: number) => version + 1, 0);

  const cancelPendingFocus = useCallback(() => {
    requestVersion.current++;
    pending.current = null;
  }, []);

  // Capture the intent before an asynchronous action. A later interaction or
  // action invalidates it, so slow work cannot take focus back from the user.
  const prepareFocus = useCallback((): FocusRequest => {
    cancelPendingFocus();
    const version = requestVersion.current;
    return (target, isCurrent = () => true) => {
      if (
        !mounted.current ||
        version !== requestVersion.current ||
        !isCurrent()
      )
        return;
      pending.current = { target, isCurrent };
      scheduleCommit();
    };
  }, [cancelPendingFocus]);

  const requestFocus = useCallback<FocusRequest>(
    (target, isCurrent) => prepareFocus()(target, isCurrent),
    [prepareFocus],
  );

  useLayoutEffect(() => {
    mounted.current = true;
    const onFocus = (event: FocusEvent) => {
      // Removing/disabling the initiating control can return focus to the
      // document. That is part of the commit, rather than a new user choice.
      if (
        event.target !== document.body &&
        event.target !== document.documentElement
      )
        cancelPendingFocus();
    };
    document.addEventListener("focusin", onFocus, true);
    document.addEventListener("pointerdown", cancelPendingFocus, true);
    document.addEventListener("keydown", cancelPendingFocus, true);
    return () => {
      mounted.current = false;
      cancelPendingFocus();
      document.removeEventListener("focusin", onFocus, true);
      document.removeEventListener("pointerdown", cancelPendingFocus, true);
      document.removeEventListener("keydown", cancelPendingFocus, true);
    };
  }, [cancelPendingFocus]);

  useLayoutEffect(() => {
    const request = pending.current;
    if (request === null) return;
    if (!request.isCurrent()) {
      cancelPendingFocus();
      return;
    }
    const target = request.target();
    if (target === null || !target.isConnected || target.matches(":disabled"))
      return;
    pending.current = null;
    target.focus();
  });

  return { requestFocus, prepareFocus, cancelPendingFocus };
}
