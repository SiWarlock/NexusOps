// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
afterEach(cleanup);
import { ProfileChangeControl } from "./ProfileChangeControl";
import { ReadOnlyProvider, type ConnectionStatus } from "../../connection/read-only";
import { MockGatewayPort } from "../../gateway-client/mock";

const CONNECTED: ConnectionStatus = { connection: "connected", version: "compatible" };
const DEGRADED: ConnectionStatus = { connection: "connecting", version: "unknown" };

function renderControl(opts: {
  status?: ConnectionStatus;
  gateway?: MockGatewayPort;
  sessionId?: string;
  sessionStatus?: string;
  currentProfileId?: string | null;
}) {
  const gateway = opts.gateway ?? new MockGatewayPort();
  render(
    <ReadOnlyProvider value={opts.status ?? CONNECTED}>
      <ProfileChangeControl
        gateway={gateway}
        sessionId={opts.sessionId ?? "session_fixture_1"}
        status={opts.sessionStatus ?? "active"}
        currentProfileId={opts.currentProfileId ?? "ep_claude_default"}
      />
    </ReadOnlyProvider>,
  );
  return { gateway };
}

const openButton = () =>
  screen.getByRole("button", { name: /change profile/i }) as HTMLButtonElement;

describe("ProfileChangeControl (WAVE-1 W1-C-a — session.profile_change)", () => {
  it("profile_change_control_fetches_profiles_on_open", async () => {
    // spec(§6.1) — the picker fetches get_execution_profiles ON-DEMAND (when opened, not eagerly), then
    // renders the profile choices (dogfoods the ui-085 read transport).
    const { gateway } = renderControl({});
    const spy = vi.spyOn(gateway, "get_execution_profiles");
    expect(spy).not.toHaveBeenCalled(); // not eager
    fireEvent.click(openButton());
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    expect(await screen.findByRole("combobox")).toBeTruthy();
  });

  it("profile_change_control_preselects_default", async () => {
    // spec(picker UX) — the picker pre-selects the is_default profile (the daemon's seeded default).
    renderControl({});
    fireEvent.click(openButton());
    const select = (await screen.findByRole("combobox")) as HTMLSelectElement;
    expect(select.value).toBe("ep_claude_default"); // the Mock's is_default profile
  });

  it("profile_change_control_shows_current_profile", async () => {
    // spec(W1-C-a) — the picker shows the session's CURRENT execution_profile_id (what the user changes FROM).
    renderControl({ currentProfileId: "ep_old_profile" });
    fireEvent.click(openButton());
    await screen.findByRole("combobox");
    expect(screen.getByText(/current/i).textContent).toContain("ep_old_profile");
  });

  it("profile_change_control_marks_needs_credential", async () => {
    // spec(§2.8/§15 #8) — a has_credential:false profile is surfaced "needs credential" + NOT selectable
    // (a disabled, annotated option) — the user can't pick a profile that has no usable credential.
    renderControl({});
    fireEvent.click(openButton());
    await screen.findByRole("combobox");
    const option = screen.getByRole("option", { name: /needs credential/i }) as HTMLOptionElement;
    expect(option.disabled).toBe(true);
  });

  it("profile_change_control_submits_selected_profile_for_row", async () => {
    // spec(§6.1/§9.1) — picking a (credentialed) profile + submitting calls submit_action with a
    // session.profile_change request carrying THIS session's resource_ref + the CHOSEN execution_profile_id.
    const { gateway } = renderControl({ sessionId: "sess_42" });
    const spy = vi.spyOn(gateway, "submit_action");
    fireEvent.click(openButton());
    await screen.findByRole("combobox");
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "ep_claude_haiku" } });
    const submit = screen.getByRole("button", { name: /submit profile change/i });
    fireEvent.click(submit);
    fireEvent.click(submit); // a rapid 2nd click must be blocked by the synchronous inFlight guard
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    const req = spy.mock.calls[0]![0];
    expect(req.action_type).toBe("session.profile_change");
    expect(req.resource_refs).toEqual([{ type: "session", id: "sess_42" }]);
    expect(req.inputs).toEqual({ execution_profile_id: "ep_claude_haiku" });
  });

  it("profile_change_control_submit_disabled_when_no_credentialed_profile", async () => {
    // spec(§15 #8) — when the picker has NO credentialed profile, nothing is submittable: the preselect
    // lands on "" (NEVER an un-credentialed id) and Submit stays disabled, so a credential-less change
    // can't fire even if the user clicks. (Pairs the preselect-fallback + submit-guard fix.)
    const gw = new MockGatewayPort();
    vi.spyOn(gw, "get_execution_profiles").mockResolvedValue({
      profiles: [
        {
          execution_profile_id: "ep_nocred",
          provider: "openai",
          harness: "codex",
          model: null,
          account_alias: null,
          status: "misconfigured",
          is_default: true,
          has_credential: false,
        },
      ],
    });
    const spy = vi.spyOn(gw, "submit_action");
    renderControl({ gateway: gw });
    fireEvent.click(openButton());
    await screen.findByRole("option", { name: /needs credential/i });
    const submit = screen.getByRole("button", { name: /submit profile change/i }) as HTMLButtonElement;
    expect(submit.disabled).toBe(true);
    fireEvent.click(submit);
    expect(spy).not.toHaveBeenCalled();
  });

  it("profile_change_control_disabled_when_cannot_submit_or_gated", () => {
    // spec(forbidden #6 / L2 gate / W1-C-a go-live gate) — the opener is disabled unless canSubmitIntent
    // AND mutationsEnabled AND enabledProfileChange (defense-in-depth to the daemon risk-2 approval).
    renderControl({ status: DEGRADED });
    expect(openButton().disabled).toBe(true);
    cleanup();
    renderControl({ gateway: new MockGatewayPort({ mutationsEnabled: false }) });
    expect(openButton().disabled).toBe(true);
    cleanup();
    renderControl({ gateway: new MockGatewayPort({ enabledProfileChange: false }) });
    expect(openButton().disabled).toBe(true);
  });

  it("profile_change_control_hidden_for_terminal_sessions", () => {
    // spec(§5.1) — a terminal session can't change profile → the control renders nothing (reuse isEndedSession).
    renderControl({ sessionStatus: "completed" });
    expect(screen.queryByRole("button", { name: /change profile/i })).toBeNull();
  });

  it("profile_change_notice_non_optimistic", async () => {
    // spec(§11.7 / §6.2 / LESSON 16/22) — risk-2 ENTERS the approval flow (not immediate): on ok → a
    // "submitted for approval" notice (the daemon-reported status, NEVER a fabricated "changed"); on a
    // §6.4 rejection → the VERBATIM code; a transport fault → honest degrade (instanceof Error).
    // ok → approval notice (the Mock submit_action returns a non-terminal ack, never "changed").
    const okGw = new MockGatewayPort();
    renderControl({ gateway: okGw });
    fireEvent.click(openButton());
    await screen.findByRole("combobox");
    fireEvent.click(screen.getByRole("button", { name: /submit profile change/i }));
    const okNotice = await screen.findByRole("status");
    expect(okNotice.textContent).toMatch(/approval/i);
    expect(okNotice.textContent).not.toMatch(/changed/i);

    cleanup();
    // §6.4 rejection → the verbatim code (get_execution_profiles is a READ — unaffected by mutationError).
    const rejecting = new MockGatewayPort({ mutationError: { code: "policy_denied" } });
    renderControl({ gateway: rejecting });
    fireEvent.click(openButton());
    await screen.findByRole("combobox");
    fireEvent.click(screen.getByRole("button", { name: /submit profile change/i }));
    expect((await screen.findByRole("status")).textContent).toMatch(/policy_denied/i);

    cleanup();
    // transport fault (an Error instance) → honest degrade (not a faked wire code).
    const faulting = new MockGatewayPort();
    vi.spyOn(faulting, "submit_action").mockRejectedValue(new Error("socket down"));
    renderControl({ gateway: faulting });
    fireEvent.click(openButton());
    await screen.findByRole("combobox");
    fireEvent.click(screen.getByRole("button", { name: /submit profile change/i }));
    const faultNotice = await screen.findByRole("status");
    expect(faultNotice.textContent).toMatch(/unavailable|couldn't/i);
    expect(faultNotice.textContent).not.toMatch(/changed/i);
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

describe("enabledProfileChange — held-flip discipline (no production go-live, the lead's standing constraint)", () => {
  it("no_production_file_enables_profile_change", () => {
    // spec(W1-C-a / cat-1) — profile-change is HELD pending the user's cat-1 sign-off: NO production file
    // constructs a port with `enabledProfileChange: true` (the Mock default-ON is an `=` assignment, not a
    // `:` option, so it isn't matched). Mirrors the enabledSessionKill/enabledSessionLaunch held-flip pins.
    const offenders = walk("src").filter((f) =>
      /enabledProfileChange\s*:\s*true/.test(readFileSync(f, "utf8")),
    );
    expect(offenders).toEqual([]);
  });
});
