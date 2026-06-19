// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { cleanup, render, waitFor } from "@testing-library/react";

// xterm renders to canvas/DOM — NOT deterministically testable in jsdom (its
// APPEARANCE is the visual gate, Lesson 10/12). Mock it to assert the host's
// STRUCTURE: the consumer's sink is wired to term.write, and there is NO input
// path (term.onData is never wired to a gateway write — safety #9). A CLASS mock
// (xterm's Terminal/FitAddon are `new`-ed); shared spies via vi.hoisted so they
// exist before the (hoisted) vi.mock factory runs.
const h = vi.hoisted(() => {
  const writes: Uint8Array[] = [];
  const ctorOpts: unknown[] = [];
  const writeSpy = vi.fn((b: Uint8Array) => {
    writes.push(b);
  });
  const onDataSpy = vi.fn(() => ({ dispose: vi.fn() }));
  const disposeSpy = vi.fn();
  class FakeTerminal {
    open = vi.fn();
    write = writeSpy;
    loadAddon = vi.fn();
    dispose = disposeSpy;
    onData = onDataSpy;
    resize = vi.fn();
    constructor(opts: unknown) {
      ctorOpts.push(opts);
    }
  }
  return { writes, ctorOpts, writeSpy, onDataSpy, disposeSpy, FakeTerminal };
});
vi.mock("@xterm/xterm", () => ({ Terminal: h.FakeTerminal }));
vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit = vi.fn();
    activate = vi.fn();
    dispose = vi.fn();
  },
}));

import { TerminalDisplay } from "./TerminalDisplay";
import { MockGatewayPort } from "../../gateway-client/mock";

beforeEach(() => {
  h.writes.length = 0;
  h.ctorOpts.length = 0;
  h.writeSpy.mockClear();
  h.onDataSpy.mockClear();
  h.disposeSpy.mockClear();
});
afterEach(cleanup);

describe("TerminalDisplay xterm host (§6.4 / display-only #9)", () => {
  it("terminaldisplay_writes_decoded_bytes_to_xterm", async () => {
    // the fixture stream pumps through subscribe_terminal → the consumer →
    // term.write (the sink). The first frame decodes to its raw PTY bytes.
    render(<TerminalDisplay gateway={new MockGatewayPort()} terminalId="t1" />);
    await waitFor(() => expect(h.writeSpy).toHaveBeenCalled());
    expect(new TextDecoder().decode(h.writes[0]!)).toBe("$ cargo build\r\n");
  });

  it("terminal_host_has_no_input_path", async () => {
    // safety #9 pin1: the host wires only an OUTPUT sink — NO keystroke path to the
    // PTY. term.onData (xterm's input event) is NEVER wired to a gateway write (the
    // GatewayPort has no terminal-input method at all), and xterm stdin is disabled.
    render(<TerminalDisplay gateway={new MockGatewayPort()} terminalId="t1" />);
    await waitFor(() => expect(h.writeSpy).toHaveBeenCalled());
    expect(h.onDataSpy).not.toHaveBeenCalled();
    expect(h.ctorOpts[0]).toEqual(
      expect.objectContaining({ disableStdin: true }),
    );
  });

  it("disposes_the_terminal_on_unmount", async () => {
    // the effect cleanup releases the xterm instance (no leak on remount/navigation).
    const { unmount } = render(
      <TerminalDisplay gateway={new MockGatewayPort()} terminalId="t1" />,
    );
    await waitFor(() => expect(h.writeSpy).toHaveBeenCalled());
    unmount();
    expect(h.disposeSpy).toHaveBeenCalled();
  });
});
