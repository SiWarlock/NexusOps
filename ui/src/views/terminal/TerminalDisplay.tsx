import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import type { GatewayPort } from "../../gateway-client/types";
import { consumeTerminalFrame, initTerminalState } from "./terminal-stream";

/**
 * The xterm.js terminal host (§6.4 Terminal Channel / 6.3d). DISPLAY-ONLY by
 * safety #9: it mounts an xterm Terminal, subscribes to the daemon's terminal
 * OUTPUT stream via `subscribe_terminal`, and writes the decoded bytes to the
 * terminal (the consumer's sink = `term.write`). There is **NO input path** —
 * `term.onData` is NEVER wired to a gateway write, and xterm's own stdin is
 * disabled (`disableStdin`); sending keystrokes to the PTY is a P4 mutation
 * surface, not this slice. Terminal/session STATUS is never derived from these
 * bytes — that is the Session projection's job (see SessionTerminal).
 *
 * xterm renders to canvas/DOM (not deterministically unit-testable in jsdom) — its
 * APPEARANCE is verified by the visual gate (Lesson 10/12); the host's structure
 * (sink wired, no input path) is unit-tested with xterm mocked.
 */
export function TerminalDisplay({
  gateway,
  terminalId,
}: {
  gateway: GatewayPort;
  terminalId: string;
}) {
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    // Ground xterm's palette in the live Graphite Arc theme tokens (so the visual
    // gate matches the kit prototype); jsdom returns "" for custom props → the
    // literal fallbacks apply (harmless in tests, where xterm is mocked anyway).
    const cs = getComputedStyle(el);
    const cssVar = (name: string, fallback: string) =>
      cs.getPropertyValue(name).trim() || fallback;

    const term = new Terminal({
      disableStdin: true, // display-only #9 — never capture keystrokes for the PTY
      cursorBlink: false,
      convertEol: false,
      fontSize: 12.5,
      fontFamily: cssVar("--font-mono", 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace'),
      theme: {
        background: cssVar("--surface-sunken", "#0b0e14"),
        foreground: cssVar("--text-secondary", "#c5ccd6"),
      },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(el);
    fit.fit();

    // OUTPUT sink only — bytes flow daemon → xterm; nothing flows back (display #9).
    let cancelled = false;
    const sink = { write: (bytes: Uint8Array) => term.write(bytes) };
    void (async () => {
      let st = initTerminalState();
      for await (const frame of gateway.subscribe_terminal(terminalId)) {
        if (cancelled) break;
        st = consumeTerminalFrame(st, frame, sink);
      }
    })();

    return () => {
      cancelled = true;
      term.dispose();
    };
  }, [gateway, terminalId]);

  return (
    <div
      ref={ref}
      data-testid="terminal-xterm"
      role="group"
      aria-label="Terminal output"
      style={{ height: "100%", width: "100%" }}
    />
  );
}
