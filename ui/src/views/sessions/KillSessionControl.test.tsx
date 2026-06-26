// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
afterEach(cleanup);
import { KillSessionControl } from "./KillSessionControl";
import { ReadOnlyProvider, type ConnectionStatus } from "../../connection/read-only";
import { MockGatewayPort } from "../../gateway-client/mock";
import { ENDED_SESSION_STATUSES } from "../terminal/session-lifecycle";

const CONNECTED: ConnectionStatus = { connection: "connected", version: "compatible" };
const DEGRADED: ConnectionStatus = { connection: "connecting", version: "unknown" };

function renderKill(opts: {
  status?: ConnectionStatus;
  gateway?: MockGatewayPort;
  sessionId?: string;
  sessionStatus?: string;
}) {
  const gateway = opts.gateway ?? new MockGatewayPort();
  render(
    <ReadOnlyProvider value={opts.status ?? CONNECTED}>
      <KillSessionControl
        gateway={gateway}
        sessionId={opts.sessionId ?? "session_fixture_1"}
        status={opts.sessionStatus ?? "active"}
      />
    </ReadOnlyProvider>,
  );
  return { gateway };
}

// The button's visible label is the accessible name (the kit Button has no aria-label prop). Both
// states carry "kill" ("Kill" ↔ "Confirm kill?") so /kill/i matches regardless of the confirm swap.
const killButton = () =>
  screen.getByRole("button", { name: /kill/i }) as HTMLButtonElement;

describe("KillSessionControl (WAVE-1 Slice B — session.kill)", () => {
  it("kill_control_submits_session_kill_for_the_row", async () => {
    // spec(§6.1/§9.1) — Kill → confirm → submit_action with a session.kill request whose resource_ref
    // targets THIS row's session_id (execute_kill is NaturalResourceRef-keyed, session_executor.rs).
    const { gateway } = renderKill({ sessionId: "sess_42", sessionStatus: "active" });
    const spy = vi.spyOn(gateway, "submit_action");
    fireEvent.click(killButton()); // arm confirm
    fireEvent.click(killButton()); // commit
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    const req = spy.mock.calls[0]![0];
    expect(req.action_type).toBe("session.kill");
    expect(req.resource_refs).toEqual([{ type: "session", id: "sess_42" }]);
    expect(req.inputs).toEqual({});
  });

  it("kill_control_requires_a_confirm_click_before_submitting", () => {
    // spec(destructive two-step, Q4) — a single click does NOT submit (guards an accidental stop of a
    // live agent); it arms a "Confirm?" affordance that the 2nd click commits. No blocking JS dialog (#dialogs).
    const { gateway } = renderKill({ sessionStatus: "active" });
    const spy = vi.spyOn(gateway, "submit_action");
    fireEvent.click(killButton());
    expect(spy).not.toHaveBeenCalled();
    expect(killButton().textContent).toMatch(/confirm/i);
  });

  it("kill_control_disabled_when_cannot_submit_intent", () => {
    // spec(forbidden #6) — the fail-safe READ-ONLY/degraded gate disables the destructive affordance.
    renderKill({ status: DEGRADED, sessionStatus: "active" });
    expect(killButton().disabled).toBe(true);
  });

  it("kill_control_disabled_when_mutations_not_enabled", () => {
    // spec(L2 gate) — kill is a mutation; the live L2 seam (mutationsEnabled) gates the control
    // (defense-in-depth; the daemon Gateway is the INV-SEC-1 chokepoint).
    renderKill({
      gateway: new MockGatewayPort({ mutationsEnabled: false }),
      sessionStatus: "active",
    });
    expect(killButton().disabled).toBe(true);
  });

  it("kill_control_disabled_when_session_kill_not_enabled", () => {
    // spec(Slice B go-live gate) — the per-action enabledSessionKill gate (default OFF in production)
    // disables the control even with mutationsEnabled on — kill is HELD for a cat-1 sign-off.
    renderKill({
      gateway: new MockGatewayPort({ enabledSessionKill: false }),
      sessionStatus: "active",
    });
    expect(killButton().disabled).toBe(true);
  });

  it("kill_control_hidden_for_terminal_sessions", () => {
    // spec(§5.1) — EVERY terminal session has nothing to kill → the control renders nothing (no dead
    // control). Iterates the canonical ENDED set so a new terminal status is auto-covered (Lesson §5).
    for (const terminal of ENDED_SESSION_STATUSES) {
      renderKill({ sessionStatus: terminal });
      expect(screen.queryByRole("button", { name: /kill/i })).toBeNull();
      cleanup();
    }
  });

  it("kill_control_double_submit_guarded", async () => {
    // spec(ui-083 review) — the synchronous inFlight ref blocks a rapid double-fire (never two kills);
    // after confirm, a 3rd rapid click must not fire a 2nd submit.
    const { gateway } = renderKill({ sessionStatus: "active" });
    const spy = vi.spyOn(gateway, "submit_action");
    fireEvent.click(killButton()); // arm confirm
    fireEvent.click(killButton()); // commit (submit #1)
    fireEvent.click(killButton()); // rapid 3rd click
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
  });

  it("kill_control_error_only_no_persistent_success", async () => {
    // spec(§11.7 / LESSON 16/22) — on a §6.4 rejection → an honest notice with the VERBATIM code; on
    // SUCCESS → NO persistent notice (the killed row transitions via the live Session nudge — that IS
    // the success signal). A transport fault degrades honestly (classify by instanceof Error).
    const rejecting = new MockGatewayPort({ mutationError: { code: "fencing_conflict" } });
    renderKill({ gateway: rejecting, sessionStatus: "active" });
    fireEvent.click(killButton());
    fireEvent.click(killButton());
    expect((await screen.findByRole("status")).textContent).toMatch(/fencing_conflict/i);

    cleanup();
    // a successful kill shows NO persistent status notice (and resets out of the confirm state).
    renderKill({ sessionStatus: "active" });
    fireEvent.click(killButton());
    fireEvent.click(killButton());
    await waitFor(() => expect(killButton().textContent).toMatch(/^kill/i)); // settled back to Kill
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("kill_control_re_arm_clears_a_stale_error", async () => {
    // spec(§11.7 hygiene) — after a FAILED kill (error banner shown, button reset to "Kill"), re-arming
    // the confirm CLEARS the stale error so the prior failure isn't shown next to the new "Confirm kill?"
    // (the arm path clears it, mirroring the commit path). Guards a misleading-stale-error regression.
    const rejecting = new MockGatewayPort({ mutationError: { code: "fencing_conflict" } });
    renderKill({ gateway: rejecting, sessionStatus: "active" });
    fireEvent.click(killButton()); // arm
    fireEvent.click(killButton()); // commit → rejects → error shown, resets to "Kill"
    expect((await screen.findByRole("status")).textContent).toMatch(/fencing_conflict/i);
    fireEvent.click(killButton()); // re-arm → must clear the stale error
    expect(killButton().textContent).toMatch(/confirm/i);
    expect(screen.queryByRole("status")).toBeNull();
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

describe("enabledSessionKill — held-flip discipline (no production go-live, the lead's standing constraint)", () => {
  it("no_production_file_enables_session_kill", () => {
    // spec(Slice B / cat-1) — agent-KILL is HELD pending the user's cat-1 sign-off + visual gate: NO
    // production file constructs a port with `enabledSessionKill: true`. The Mock's default-ON is an `=`
    // assignment (mock.ts), not a `:` construction option, so it isn't matched. When the user signs off,
    // a SINGLE confined flip site (the Shell, the ui-075 precedent) updates this guard — never a silent
    // ride of `mutationsEnabled`. Mirrors the enabledSessionLaunch held-flip pin (LaunchAgentControl.test).
    const offenders = walk("src").filter((f) =>
      /enabledSessionKill\s*:\s*true/.test(readFileSync(f, "utf8")),
    );
    expect(offenders).toEqual([]);
  });
});
