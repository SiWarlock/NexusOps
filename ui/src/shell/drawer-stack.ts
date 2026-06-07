// The right drawer stack — a LIFO reducer (§11). One drawer is visible at a time
// (the top of the stack); ESC pops the top (mapped at the component level).
export interface Drawer {
  kind: string;
  props?: Record<string, unknown>;
}

export type DrawerStackState = Drawer[];

export type DrawerAction =
  | { type: "push"; drawer: Drawer }
  | { type: "pop" }
  | { type: "replace"; drawer: Drawer }
  | { type: "clear" };

export function drawerStackReducer(
  state: DrawerStackState,
  action: DrawerAction,
): DrawerStackState {
  switch (action.type) {
    case "push":
      return [...state, action.drawer];
    case "pop":
      return state.slice(0, -1);
    case "replace":
      // Swap the top only (depth unchanged); on an empty stack, open the drawer.
      return state.length === 0
        ? [action.drawer]
        : [...state.slice(0, -1), action.drawer];
    case "clear":
      return [];
  }
}

/** The currently-visible drawer (top of stack), or undefined when none is open. */
export function topDrawer(state: DrawerStackState): Drawer | undefined {
  return state[state.length - 1];
}
