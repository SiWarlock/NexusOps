// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
afterEach(cleanup);
import { SessionDriveControls } from "./SessionDriveControls";
import { ReadOnlyProvider, type ConnectionStatus } from "../../connection/read-only";
import { MockGatewayPort } from "../../gateway-client/mock";
import { ENDED_SESSION_STATUSES } from "../terminal/session-lifecycle";

const CONNECTED: ConnectionStatus = { connection: "connected", version: "compatible" };
const DEGRADED: ConnectionStatus = { connection: "connecting", version: "unknown" };

function renderDrive(opts: {
  status?: ConnectionStatus;
  gateway?: MockGatewayPort;
  sessionId?: string;
  sessionStatus?: string;
}) {
  const gateway = opts.gateway ?? new MockGatewayPort();
  const utils = render(
    <ReadOnlyProvider value={opts.status ?? CONNECTED}>
      <SessionDriveControls
        gateway={gateway}
        sessionId={opts.sessionId ?? "session_fixture_1"}
        status={opts.sessionStatus ?? "active"}
      />
    </ReadOnlyProvider>,
  );
  return { gateway, ...utils };
}

const pauseBtn = () =>
  screen.getByRole("button", { name: /pause monitoring/i }) as HTMLButtonElement;
const resumeBtn = () =>
  screen.getByRole("button", { name: /resume monitoring/i }) as HTMLButtonElement;
const sendBtn = () => screen.getByRole("button", { name: /^send$/i }) as HTMLButtonElement;

describe("SessionDriveControls (WAVE-1 W1-C-b — send_message / pause / resume)", () => {
  it("pause_resume_labeled_monitoring_not_suspend", () => {
    // spec(daemon LESSON 71 — LOAD-BEARING honesty) — MVP pause is SOFT (gates the cockpit drive/observe
    // loop, does NOT OS-suspend the agent → INV-SEC-1 holds). The labels MUST read "Pause monitoring"/
    // "Resume monitoring" — NEVER "stop"/"suspend"/"halt" (a user must not believe Pause halts the agent).
    const { container } = renderDrive({});
    expect(pauseBtn()).toBeTruthy();
    expect(resumeBtn()).toBeTruthy();
    const forbidden = /\b(stop|suspend|halt)\b/i;
    expect(container.textContent).not.toMatch(forbidden);
    // tooltips/aria too — a misleading title/aria-label would mislead a hovering user just as a label would
    // (the honesty pin must cover the FULL surface, not only visible text — security-reviewer coverage gap).
    for (const el of container.querySelectorAll("[title], [aria-label]")) {
      expect(el.getAttribute("title") ?? "").not.toMatch(forbidden);
      expect(el.getAttribute("aria-label") ?? "").not.toMatch(forbidden);
    }
  });

  it("both_pause_and_resume_always_available_no_toggle", () => {
    // spec(no UI-readable paused state) — the soft pause isn't projected (no `paused` field), so the MVP
    // shows BOTH actions (no stateful toggle — the UI can't know if a session is currently paused).
    renderDrive({});
    expect(pauseBtn()).toBeTruthy();
    expect(resumeBtn()).toBeTruthy();
  });

  it("send_message_blocks_empty", () => {
    // spec(§6.3 honest UI) — an empty/whitespace message blocks submit (no empty intent; the daemon
    // FromInputs rejects it too, but the UI blocks first). Send is disabled until a non-blank message.
    const { gateway } = renderDrive({});
    const spy = vi.spyOn(gateway, "submit_action");
    expect(sendBtn().disabled).toBe(true); // empty
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "   " } });
    expect(sendBtn().disabled).toBe(true); // whitespace-only
    fireEvent.click(sendBtn());
    expect(spy).not.toHaveBeenCalled();
  });

  it("send_message_submits_trimmed_for_the_session", async () => {
    // spec(§6.1/§9.1) — Send submits buildSendMessageActionRequest with the trimmed non-empty message +
    // THIS session's resource_ref.
    const { gateway } = renderDrive({ sessionId: "sess_42" });
    const spy = vi.spyOn(gateway, "submit_action");
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "  hello  " } });
    expect(sendBtn().disabled).toBe(false);
    fireEvent.click(sendBtn());
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    const req = spy.mock.calls[0]![0];
    expect(req.action_type).toBe("session.send_message");
    expect(req.resource_refs).toEqual([{ type: "session", id: "sess_42" }]);
    expect(req.inputs).toEqual({ message: "hello" });
    // the input CLEARS on a successful send (the only path that resets `message`).
    await waitFor(() =>
      expect((screen.getByRole("textbox") as HTMLInputElement).value).toBe(""),
    );
  });

  it("pause_and_resume_submit_their_action_types", async () => {
    // spec(§6.1/§9.1) — Pause monitoring → session.pause; Resume monitoring → session.resume (each for
    // THIS session).
    const { gateway } = renderDrive({ sessionId: "sess_42" });
    const spy = vi.spyOn(gateway, "submit_action");
    fireEvent.click(pauseBtn());
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    expect(spy.mock.calls[0]![0].action_type).toBe("session.pause");
    expect(spy.mock.calls[0]![0].resource_refs).toEqual([{ type: "session", id: "sess_42" }]);
    await waitFor(() => expect(resumeBtn().disabled).toBe(false)); // in-flight cleared
    fireEvent.click(resumeBtn());
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(2));
    expect(spy.mock.calls[1]![0].action_type).toBe("session.resume");
    expect(spy.mock.calls[1]![0].resource_refs).toEqual([{ type: "session", id: "sess_42" }]);
  });

  it("drive_controls_disabled_when_cannot_submit_or_gated", () => {
    // spec(forbidden #6 / L2 gate / the Set gate) — every drive control is disabled unless canSubmitIntent
    // AND mutationsEnabled AND the action_type is in enabledSessionControls.
    renderDrive({ status: DEGRADED });
    // baseEnabled gates ALL three affordances — pin each (not just pause).
    expect(pauseBtn().disabled).toBe(true);
    expect(resumeBtn().disabled).toBe(true);
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "hi" } });
    expect(sendBtn().disabled).toBe(true);
    cleanup();
    renderDrive({ gateway: new MockGatewayPort({ mutationsEnabled: false }) });
    expect(pauseBtn().disabled).toBe(true);
    expect(resumeBtn().disabled).toBe(true);
    cleanup();
    renderDrive({ gateway: new MockGatewayPort({ enabledSessionControls: new Set() }) });
    expect(pauseBtn().disabled).toBe(true);
    expect(resumeBtn().disabled).toBe(true);
  });

  it("drive_controls_hidden_for_terminal_sessions", () => {
    // spec(§5.1) — EVERY terminal session can't be driven → the controls render nothing. Iterates the
    // canonical ENDED set so a new terminal status is auto-covered (reuse isEndedSession, Lesson §5).
    for (const terminal of ENDED_SESSION_STATUSES) {
      renderDrive({ sessionStatus: terminal });
      expect(screen.queryByRole("button", { name: /pause monitoring/i })).toBeNull();
      cleanup();
    }
  });

  it("drive_notice_non_optimistic", async () => {
    // spec(§6.2/§11.7/LESSON 16/22) — risk≥1 ENTERS the approval flow: on ok → a "submitted for approval"
    // notice (the daemon-reported status, NEVER "sent"/"paused"); §6.4 rejection → VERBATIM code; transport
    // fault → honest degrade (instanceof Error).
    renderDrive({});
    fireEvent.click(pauseBtn());
    const okNotice = await screen.findByRole("status");
    expect(okNotice.textContent).toMatch(/approval/i);
    expect(okNotice.textContent).not.toMatch(/\bpaused\b|\bsent\b/i);

    cleanup();
    const rejecting = new MockGatewayPort({ mutationError: { code: "policy_denied" } });
    renderDrive({ gateway: rejecting });
    fireEvent.click(pauseBtn());
    expect((await screen.findByRole("status")).textContent).toMatch(/policy_denied/i);

    cleanup();
    const faulting = new MockGatewayPort();
    vi.spyOn(faulting, "submit_action").mockRejectedValue(new Error("socket down"));
    renderDrive({ gateway: faulting });
    fireEvent.click(pauseBtn());
    expect((await screen.findByRole("status")).textContent).toMatch(/unavailable|couldn't/i);
  });

  it("drive_inflight_double_submit_guarded", async () => {
    // spec(ui-083 review) — the synchronous inFlight ref blocks a rapid double-fire (never two intents).
    const { gateway } = renderDrive({});
    const spy = vi.spyOn(gateway, "submit_action");
    const p = pauseBtn();
    fireEvent.click(p);
    fireEvent.click(p);
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
  });
});

function walk(dir: string): string[] {
  const out: string[] = [];
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) out.push(...walk(p));
    else if (/\.(ts|tsx)$/.test(e.name) && !/\.test\.(ts|tsx)$/.test(e.name)) out.push(p);
  }
  return out;
}

describe("enabledSessionControls — held-flip discipline (no production go-live; the enabledPrMutations mirror)", () => {
  it("no_production_file_enables_session_controls", () => {
    // spec(cat-1 / the lead's standing constraint) — the drive controls are HELD pending a USER cat-1
    // sign-off: NO production file constructs a port with a NON-EMPTY enabledSessionControls VALUE. The
    // scan flags any `enabledSessionControls: <value>` that is neither a `ReadonlySet<…>` TYPE decl nor the
    // empty `new Set()` (the Mock's full-set default is an `=` assignment, not a `:` option — dev/test only).
    const offenders = walk("src").filter((f) => {
      const src = readFileSync(f, "utf8");
      for (const m of src.matchAll(/enabledSessionControls\s*:\s*([^\n;,]+)/g)) {
        const value = m[1]!.trim();
        if (
          !value.startsWith("ReadonlySet") &&
          !/^new Set\s*<[^>]*>?\s*\(\s*\)/.test(value) &&
          !/^new Set\s*\(\s*\)/.test(value)
        ) {
          return true;
        }
      }
      return false;
    });
    expect(offenders).toEqual([]);
  });
});
