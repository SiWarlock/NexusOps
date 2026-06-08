// The UI content-view navigation history (§11.2 TopBar back/forward nav model).
// PURE UI state over the content-view SELECTION — NOT a daemon mutation, NOT a
// provisional contract, NOT a Gateway intent (a view/scope selection, so no
// canSubmitIntent gate). Same family as active-project (Lesson §13): a pure model
// + a thin React wrapper. Browser-style semantics: navigate pushes + truncates
// any stranded forward entries; back/forward move only the cursor.
import { useCallback, useReducer } from "react";

/** The four content views the main surface can show (§11.2). Settings is reached
 *  via the TopBar, but it's a uniform history entry like the rest. */
export type ViewName = "command" | "graph" | "sessions" | "settings";

/** History as a stack + a cursor — `stack[cursor]` is the current view. */
export interface ViewHistoryState {
  stack: ViewName[];
  cursor: number;
}

export type ViewHistoryAction =
  | { type: "navigate"; view: ViewName }
  | { type: "back" }
  | { type: "forward" };

/** Fresh history: Command Center, no neighbors either side. */
export const initialViewHistory: ViewHistoryState = { stack: ["command"], cursor: 0 };

export function currentView(state: ViewHistoryState): ViewName {
  // Invariant: cursor is always in [0, stack.length - 1] (initial + every reducer
  // case preserve it), so the index never misses — the `!` is sound, not a guess.
  return state.stack[state.cursor]!;
}

export function canGoBack(state: ViewHistoryState): boolean {
  return state.cursor > 0;
}

export function canGoForward(state: ViewHistoryState): boolean {
  return state.cursor < state.stack.length - 1;
}

export function viewHistoryReducer(
  state: ViewHistoryState,
  action: ViewHistoryAction,
): ViewHistoryState {
  switch (action.type) {
    case "navigate": {
      // Collapse: re-selecting the active view is not a back-step (no duplicate).
      if (action.view === currentView(state)) return state;
      // Forward-discard: a nav from a back position strands + truncates the
      // entries ahead of the cursor (classic browser chrome).
      const stack = [...state.stack.slice(0, state.cursor + 1), action.view];
      return { stack, cursor: stack.length - 1 };
    }
    case "back":
      // Idempotent at the start — never an out-of-bounds cursor.
      return canGoBack(state) ? { stack: state.stack, cursor: state.cursor - 1 } : state;
    case "forward":
      // Idempotent at the tip.
      return canGoForward(state)
        ? { stack: state.stack, cursor: state.cursor + 1 }
        : state;
  }
}

export interface ViewHistory {
  current: ViewName;
  canBack: boolean;
  canForward: boolean;
  navigate: (view: ViewName) => void;
  back: () => void;
  forward: () => void;
}

/**
 * Drives the content view from the history reducer (mirrors active-project's
 * pure-model-plus-thin-wrapper). `navigate` is the single nav entry point for
 * every view-switch call site; `back`/`forward` operate the TopBar controls.
 */
export function useViewHistory(): ViewHistory {
  const [state, dispatch] = useReducer(viewHistoryReducer, initialViewHistory);
  return {
    current: currentView(state),
    canBack: canGoBack(state),
    canForward: canGoForward(state),
    navigate: useCallback((view: ViewName) => dispatch({ type: "navigate", view }), []),
    back: useCallback(() => dispatch({ type: "back" }), []),
    forward: useCallback(() => dispatch({ type: "forward" }), []),
  };
}
