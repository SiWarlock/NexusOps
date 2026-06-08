import { useEffect, useState } from "react";

// §11.6 / §11.1: tracks the user's prefers-reduced-motion setting for JS-GATED
// motion. The kit's global @media(prefers-reduced-motion) guard (motion.css)
// already suppresses CSS animation/transition; this hook is for the JS-driven
// motion that lands later (live-pulse / attention-beacon / overlay entrances).
const QUERY = "(prefers-reduced-motion: reduce)";

function prefersReducedMotion(): boolean {
  // matchMedia is absent under SSR / jsdom-without-a-mock — reduced-motion is an
  // explicit OS opt-in, so its absence means motion is allowed (default false).
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return false;
  }
  return window.matchMedia(QUERY).matches;
}

export function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState<boolean>(prefersReducedMotion);

  useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
      return;
    }
    // The useState lazy initializer already read the live value at mount; the
    // listener owns every subsequent change (the event's own `matches` is the
    // single source of truth — no captured reference).
    const mql = window.matchMedia(QUERY);
    const onChange = (e: MediaQueryListEvent) => setReduced(e.matches);
    mql.addEventListener("change", onChange);
    return () => mql.removeEventListener("change", onChange);
  }, []);

  return reduced;
}
