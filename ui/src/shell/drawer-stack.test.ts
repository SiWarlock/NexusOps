import { describe, it, expect } from "vitest";
import {
  drawerStackReducer,
  topDrawer,
  type DrawerStackState,
} from "./drawer-stack";

const A = { kind: "inspector" };
const B = { kind: "approval" };
const C = { kind: "settings" };

describe("drawerStackReducer", () => {
  it("drawer_stack_push_pop_lifo", () => {
    let state: DrawerStackState = [];
    state = drawerStackReducer(state, { type: "push", drawer: A });
    state = drawerStackReducer(state, { type: "push", drawer: B });
    expect(topDrawer(state)).toEqual(B); // LIFO: last pushed is on top

    state = drawerStackReducer(state, { type: "pop" });
    expect(topDrawer(state)).toEqual(A);

    state = drawerStackReducer(state, { type: "pop" });
    expect(topDrawer(state)).toBeUndefined(); // empty
    expect(state).toHaveLength(0);
  });

  it("drawer_stack_replace_and_clear", () => {
    let state: DrawerStackState = drawerStackReducer(
      drawerStackReducer([], { type: "push", drawer: A }),
      { type: "push", drawer: B },
    );
    // replace swaps the top only (B -> C), depth unchanged
    state = drawerStackReducer(state, { type: "replace", drawer: C });
    expect(topDrawer(state)).toEqual(C);
    expect(state).toHaveLength(2);

    // ESC-equivalent pops the top
    state = drawerStackReducer(state, { type: "pop" });
    expect(topDrawer(state)).toEqual(A);

    // clear empties the whole stack
    state = drawerStackReducer(state, { type: "clear" });
    expect(state).toHaveLength(0);

    // replace on an EMPTY stack opens the drawer (documented behavior)
    expect(drawerStackReducer([], { type: "replace", drawer: A })).toEqual([A]);
  });
});
