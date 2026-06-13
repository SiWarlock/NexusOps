// The pure §6.4 terminal-stream consumer — xterm-FREE so it's deterministically
// unit-testable. It decodes the frozen `terminal_output` frame's base64 `data` to
// raw PTY bytes and writes them to a sink (the xterm host wires `term.write` as the
// sink at L2). DISPLAY-ONLY by safety #9: bytes flow OUT to the sink only — they
// are NEVER read back to derive machine state, and there is no input path.
import type { TerminalOutputFrame } from "../../contracts/index";

/** A raw-byte sink. The xterm host wires `term.write`; tests wire a recorder. */
export interface TerminalSink {
  write(bytes: Uint8Array): void;
}

/** Display-only consumer state: the last `seq` seen (monotonic is ASSUMED). */
export interface TerminalConsumerState {
  lastSeq: number | null;
}

export function initTerminalState(): TerminalConsumerState {
  return { lastSeq: null };
}

/** Decode a §6.4 frame's base64 `data` to raw PTY output bytes. */
export function decodeBase64(data: string): Uint8Array {
  const bin = atob(data);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

/**
 * Consume one frozen `terminal_output` frame: decode its base64 `data` and write
 * the raw bytes to the sink (display-only #9 — the bytes are never read back for
 * state), then track `seq`. Monotonic `seq` is ASSUMED; a gap is NOT gap-filled or
 * synthesized — the daemon closes the connection on lag, so reconnect/re-subscribe
 * is the recovery model (P4). Returns the next state.
 */
export function consumeTerminalFrame(
  state: TerminalConsumerState,
  frame: TerminalOutputFrame,
  sink: TerminalSink,
): TerminalConsumerState {
  sink.write(decodeBase64(frame.data));
  return { ...state, lastSeq: frame.seq };
}
