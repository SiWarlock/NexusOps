// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useReducedMotion } from "./useReducedMotion";

// A controllable matchMedia stub (jsdom ships none) — tracks listeners so the
// leak check is real, and lets a test flip `matches` + fire a change event.
// `mockReturnValue` returns the SAME mql object on every call, so the listener
// set faithfully reflects the hook's add/remove (the leak check is load-bearing
// on that — a per-call new object would hide a real leak).
function installMatchMedia(initial: boolean) {
  const listeners = new Set<(e: MediaQueryListEvent) => void>();
  const mql = {
    matches: initial,
    media: "(prefers-reduced-motion: reduce)",
    onchange: null,
    addEventListener: (_t: string, cb: (e: MediaQueryListEvent) => void) =>
      listeners.add(cb),
    removeEventListener: (_t: string, cb: (e: MediaQueryListEvent) => void) =>
      listeners.delete(cb),
    addListener: (cb: (e: MediaQueryListEvent) => void) => listeners.add(cb),
    removeListener: (cb: (e: MediaQueryListEvent) => void) => listeners.delete(cb),
    dispatchEvent: () => true,
  };
  window.matchMedia = vi.fn().mockReturnValue(mql) as unknown as typeof window.matchMedia;
  return {
    listenerCount: () => listeners.size,
    fire: (matches: boolean) => {
      mql.matches = matches;
      for (const cb of listeners) cb({ matches } as MediaQueryListEvent);
    },
  };
}

afterEach(() => {
  // @ts-expect-error — clear the stub so the "unavailable" case is clean
  delete window.matchMedia;
  vi.restoreAllMocks();
});

describe("useReducedMotion", () => {
  it("returns_true_when_reduce_preferred", () => {
    installMatchMedia(true);
    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(true);
  });

  it("returns_false_when_no_preference", () => {
    installMatchMedia(false);
    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(false);
  });

  it("defaults_false_when_matchmedia_unavailable", () => {
    // no window.matchMedia (jsdom default) → motion allowed, no throw
    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(false);
  });

  it("updates_on_preference_change", () => {
    const mm = installMatchMedia(false);
    const { result, unmount } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(false);
    // a change to reduce updates the returned value
    act(() => mm.fire(true));
    expect(result.current).toBe(true);
    // listener removed on unmount (no leak)
    unmount();
    expect(mm.listenerCount()).toBe(0);
  });
});
