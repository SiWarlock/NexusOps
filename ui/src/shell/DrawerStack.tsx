import { useEffect, useReducer } from "react";
import {
  drawerStackReducer,
  topDrawer,
  type DrawerStackState,
} from "./drawer-stack";

const EMPTY: DrawerStackState = [];

/**
 * Right drawer stack region (§11). LIFO: one drawer visible at a time (the top);
 * ESC pops the top. 6.1b wires the reducer + ESC + reserves the region; the
 * drawers that push onto it (inspector, approval, etc.) arrive with the screens
 * in 6.3.
 */
export function DrawerStack() {
  const [stack, dispatch] = useReducer(drawerStackReducer, EMPTY);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") dispatch({ type: "pop" });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const top = topDrawer(stack);
  return (
    <div
      className="drawer-stack"
      aria-label="Drawer stack"
      data-drawer-depth={stack.length}
    >
      {top ? (
        <section
          className="drawer"
          role="dialog"
          aria-label={top.kind}
          data-drawer-kind={top.kind}
        />
      ) : null}
    </div>
  );
}
