import { describe, it, expect } from "vitest";
import {
  initTerminalState,
  consumeTerminalFrame,
  decodeBase64,
  type TerminalSink,
} from "./terminal-stream";
import type { TerminalOutputFrame } from "../../contracts/index";

// A recording sink — the pure consumer is xterm-FREE: it feeds decoded bytes to
// any sink (the xterm host wires `term.write` as the sink at L2). Display-only #9:
// the consumer NEVER reads bytes back for state; it only writes them out.
function recordingSink(): { writes: Uint8Array[]; sink: TerminalSink } {
  const writes: Uint8Array[] = [];
  return { writes, sink: { write: (b) => writes.push(b) } };
}

const out = (seq: number, data: string): TerminalOutputFrame => ({
  frame_type: "terminal_output",
  terminal_id: "t1",
  seq,
  data,
});

describe("terminal-stream consumer (§6.4 Terminal Channel, display-only #9)", () => {
  it("decodeBase64_decodes_raw_pty_bytes", () => {
    // base64 "aGk=" → bytes for "hi"; the §6.4 frame `data` is opaque base64.
    expect([...decodeBase64("aGk=")]).toEqual([104, 105]);
  });

  it("consumer_decodes_frozen_frame_no_invention", () => {
    // spec(§6.4 / safety #9 pin3): an output frame's base64 `data` decodes to
    // EXACTLY those bytes on the sink — the consumer never invents transcript
    // content beyond the decode (the placeholder's honest promise).
    const { writes, sink } = recordingSink();
    let st = initTerminalState();
    st = consumeTerminalFrame(st, out(0, "aGk="), sink);
    expect(writes.length).toBe(1);
    expect([...writes[0]!]).toEqual([104, 105]);
    expect(st.lastSeq).toBe(0);
  });

  it("skips_undecodable_frame_without_throwing", () => {
    // spec(§6.4): a corrupt/undecodable base64 frame is SKIPPED (display-degrade) —
    // never thrown (which would crash the well's stream loop) and never invented;
    // seq still advances so subsequent frames keep their order.
    const { writes, sink } = recordingSink();
    let st = initTerminalState();
    const bad = { frame_type: "terminal_output", terminal_id: "t1", seq: 0, data: "@@@@" } as const;
    expect(() => {
      st = consumeTerminalFrame(st, bad, sink);
    }).not.toThrow();
    expect(writes.length).toBe(0); // nothing written — couldn't decode
    expect(st.lastSeq).toBe(0); // seq still tracked
    // a following valid frame still writes
    st = consumeTerminalFrame(st, out(1, "aGk="), sink);
    expect(writes.length).toBe(1);
  });

  it("tracks_seq_writes_in_arrival_order_no_gap_fill", () => {
    // spec(§6.4): monotonic `seq` is ASSUMED; the consumer writes what it receives
    // in arrival order + tracks lastSeq. A gap (0 then 5) is NOT gap-filled or
    // synthesized — recovery is reconnect/re-subscribe (the daemon closes the
    // connection on lag), P4. Exactly N sink writes for N output frames.
    const { writes, sink } = recordingSink();
    let st = initTerminalState();
    for (const frame of [out(0, "aGk="), out(5, "b2s=")]) {
      st = consumeTerminalFrame(st, frame, sink);
    }
    expect(writes.length).toBe(2); // no synthesized fill for the 1..4 gap
    expect([...writes[1]!]).toEqual([111, 107]); // "ok" — arrival order preserved
    expect(st.lastSeq).toBe(5);
  });
});
