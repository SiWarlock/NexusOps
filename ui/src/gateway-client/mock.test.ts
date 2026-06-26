import { describe, it, expect } from "vitest";
import { MockGatewayPort } from "./mock";
import { parseDelta } from "./boundary";
import {
  DiffResult,
  Session,
  TerminalOutputFrame,
  type ProjectionDelta,
} from "../contracts/index";

describe("MockGatewayPort read surface (§14 mandate)", () => {
  it("mock_get_projection_returns_contract_valid_fixtures", async () => {
    const mock = new MockGatewayPort();
    const page = await mock.get_projection("Session");
    expect(page.rows.length).toBeGreaterThan(0);
    for (const row of page.rows) {
      // every fixture status value is a member of the frozen §5.1 Session enum
      expect(() => Session.parse(row.status)).not.toThrow();
    }
  });

  it("mock_subscribe_streams_validated_delta", async () => {
    const mock = new MockGatewayPort();
    const deltas: ProjectionDelta[] = [];
    for await (const delta of mock.subscribe({ projection: "Session" })) {
      deltas.push(delta);
      if (deltas.length >= 1) break;
    }
    expect(deltas.length).toBeGreaterThan(0);
    const delta = deltas[0]!;
    // pin the delta's real structure (not a tautological re-parse of the
    // mock's own parseDelta output)
    expect(delta.projection).toBe("Session");
    expect(delta.kind).toBe("upsert");
    expect(delta.row?.session_id).toBeTruthy();
    // and confirm it still round-trips the boundary parser end-to-end
    expect(() => parseDelta(delta)).not.toThrow();
  });

  it("mock_subscribe_terminal_yields_contract_valid_stream", async () => {
    // spec(§6.4 / §14): subscribe_terminal yields a deterministic fixture terminal
    // stream of frozen `terminal_output` frames (output ONLY — maps 1:1 to the
    // §6.4 terminal channel; the §17 exit is a daemon event→projection, NOT pushed
    // here). Every frame round-trips its frozen shadow (parse-don't-trust dogfood),
    // and the stream terminates.
    const mock = new MockGatewayPort();
    const frames: unknown[] = [];
    for await (const frame of mock.subscribe_terminal("t1")) frames.push(frame);

    expect(frames.length).toBeGreaterThan(0);
    for (const frame of frames) {
      expect(() => TerminalOutputFrame.parse(frame)).not.toThrow();
    }
  });

  it("mock_get_diff_returns_valid_diffresult", async () => {
    // spec(§6.1) — the read-only get_diff fixture is CONTRACT-shaped (a DiffResult the
    // 6.3e Code/Diff slice can consume), served so DiffResult.parse() accepts it
    // (parse-don't-trust dogfood, symmetric with get_projection/subscribe_terminal).
    const mock = new MockGatewayPort();
    const result = await mock.get_diff("wt_demo_0001", "src/main.rs");
    expect(() => DiffResult.parse(result)).not.toThrow();
    expect(result.hunks.length).toBeGreaterThan(0);
  });

  it("mock_notify_connection_state_is_guarded_setconnectionstate_stays_raw", () => {
    // spec(§11 / 054) — the mock implements the new notifyConnectionState (guarded canTransition +
    // notify, mirroring the real port so the supervisor binding works in Shell tests), while
    // setConnectionState stays the RAW, unguarded test-staging setter (the §11 DegradedBanner
    // contract — Shell.test.tsx stages `disconnected` through it). Two methods, two purposes.
    // start at the pre-handshake `connecting` (the mock defaults to `connected`) so the illegal hop
    // connecting→reconnecting actually exercises the guard.
    const mock = new MockGatewayPort({ connection: "connecting" });
    const seen: string[] = [];
    mock.onConnectionChange((s) => seen.push(s));

    // notifyConnectionState is GUARDED: connecting→reconnecting is illegal → no-op (no notify).
    mock.notifyConnectionState("Session", "reconnecting");
    expect(mock.getConnectionState()).toBe("connecting");
    expect(seen).toEqual([]);

    // setConnectionState is RAW: it stages ANY state directly + notifies (drives the degraded banner).
    mock.setConnectionState("reconnecting");
    expect(mock.getConnectionState()).toBe("reconnecting");
    expect(seen).toEqual(["reconnecting"]);
  });

  it("mock_notify_connection_state_aggregates_per_stream_worst_of", () => {
    // spec(054 / ui-059) — the mock MIRRORS the real port's per-stream worst-of aggregate so Shell tests
    // (which use the Mock) see faithful multi-stream behavior. The load-bearing non-masking invariant: a
    // HEALTHY ApprovalQueue stream must NOT clear a DEGRADED Session stream (the global stays worst-of).
    const mock = new MockGatewayPort(); // defaults to `connected`

    mock.notifyConnectionState("Session", "disconnected"); // Session down
    expect(mock.getConnectionState()).toBe("disconnected");
    mock.notifyConnectionState("ApprovalQueue", "connected"); // ApprovalQueue healthy…
    expect(mock.getConnectionState()).toBe("disconnected"); // …global stays degraded (worst-of)

    // only when ALL reported streams are connected does the aggregate recover:
    mock.notifyConnectionState("Session", "reconnecting");
    mock.notifyConnectionState("Session", "connected");
    expect(mock.getConnectionState()).toBe("connected");
  });

  it("mock_get_capabilities_reports_contract_version", async () => {
    const mock = new MockGatewayPort();
    const caps = await mock.get_capabilities();
    // literal "0.33.0" is an intentional version tripwire — it must fail loudly
    // when the frozen contract bumps (the drift test chains this to the schema).
    // Bumped 0.8.0 → 0.12.0 (main→ui merge regen) → 0.19.0 (Phase-2 Gateway freeze)
    // → 0.23.0 (Phase-3 boundary merge: §9.1 harness / §6.4 Terminal Channel)
    // → 0.28.0 (Phase-4 boundary merge: §6.3e per-hunk git actions + get_diff)
    // → 0.31.0 (L2-prep boundary merge: survival 0.29 / ApprovalQueueRow 0.30 / SessionRecovered 0.31)
    // → 0.33.0 (post-edges boundary merge: SessionFailed 0.32 + integration.connect/ExecutorKind +integration 0.33)
    // → 0.38.0 (daemon D-series UI-unblock boundary merge: D1 PullRequestRow 0.34 / D2 SessionRow recovery 0.35 /
    //           D5a mergeable+checks 0.36 / D5b-1 review vertical [ReviewState + ProjectionName +Review] 0.37 /
    //           D5b-2 github.sync_reviews 0.38).
    // → 0.42.0 (§4.7 PR-mutation boundary merge: D6 PR-card diff-stats 0.39 / D7 get_pr_diff 0.40 /
    //           D9 github.merge_pr 0.41 / D10 github.submit_review 0.42).
    // → 0.44.0 (head_sha boundary merge: 0.43 ExecutionProfileRegistered/5.3a · 0.44 PullRequestRow.head_sha
    //           exposure [the cat-1 merge/review pin source] + the ruling-A owner/repo daemon resolution).
    // → 0.45.0 (auth-bootstrap boundary merge / 083: keychain SecretStore + integration.set_live_writes
    //           [MIGRATION_18] + connect_via_gh [+ConnectViaGhStatus enum] — the PR-mutations go-live unblock).
    // → 0.46.0 (5.3b execution-profile-SECRETS boundary merge: profile-secret contract freeze + profile.set_secret
    //           / set_keychain_ref + keychain self-test + session.profile_change — NO new flat enum [42 held]).
    // → 0.47.0 (092 friendly-project-name: ProjectRescanned+name [daemon event] + proj_project_activity.name —
    //           consumed by the already-optional ProjectActivityRow.name [ui-082], NO new flat enum [42 held]).
    // → 0.48.0 (W1-prof get_execution_profiles read RPC: ProfileRow + GetExecutionProfilesResult read-RESULT
    //           types in shared/src/ipc.rs [secret-free, §15 #4] — status reuses ExecutionProfile, NO new flat enum [42 held]).
    // → 0.49.0 (W2-audit: proj_audit_trail+event_type [MIGRATION_20] + the frozen AuditEventRow [5th projection-row]
    //           — actor_label/sensitivity/event_type are struct fields / plain strings, NO new flat enum [42 held]).
    expect(caps.contract_version).toBe("0.49.0");
    expect(caps.protocol_version).toBe(1);
  });

  it("mock_enabled_pr_mutations_defaults_full_set", () => {
    // spec(ui-071/1b) — the Mock is a fully-working test/dev port: enabledPrMutations defaults to the
    // FULL PR_MUTATION_ACTION_TYPES set, so Mock-driven UI-flow tests exercise the enabled merge + review
    // paths. (Production UdsGatewayPort defaults EMPTY — the held-flip.)
    const enabled = new MockGatewayPort().enabledPrMutations;
    expect(enabled.has("github.merge_pr")).toBe(true);
    expect(enabled.has("github.submit_review")).toBe(true);
  });

  it("mock_create_session_returns_a_canned_ack", async () => {
    // spec(W1-A) — the Mock createSession returns a contract-valid ActionAck so dev/test UI flows
    // exercise the Launch path without a daemon (the daemon mints the id; the Mock cans one).
    const mock = new MockGatewayPort();
    const ack = await mock.createSession({ project_id: "proj_1", initial_prompt: "go" });
    expect(ack.action_request_id).toBeTruthy();
    expect(ack.status).toBeTruthy();
  });

  it("mock_enabled_session_launch_defaults_true", () => {
    // spec(W1-A) — the Mock is a fully-working dev/test port: enabledSessionLaunch defaults ON so
    // Mock-driven Launch flows exercise the enabled path (production UdsGatewayPort defaults OFF — held).
    expect(new MockGatewayPort().enabledSessionLaunch).toBe(true);
  });

  it("mock_enabled_session_kill_defaults_true", () => {
    // spec(Slice B) — the Mock is a fully-working dev/test port: enabledSessionKill defaults ON so
    // Mock-driven Kill flows exercise the enabled path (production UdsGatewayPort defaults OFF — held).
    expect(new MockGatewayPort().enabledSessionKill).toBe(true);
  });

  it("mock_enabled_profile_change_defaults_true", () => {
    // spec(W1-C-a) — the Mock is a fully-working dev/test port: enabledProfileChange defaults ON so
    // Mock-driven profile-change flows exercise the enabled path (production UdsGatewayPort defaults OFF).
    expect(new MockGatewayPort().enabledProfileChange).toBe(true);
  });

  it("mock_enabled_session_controls_defaults_full_set", () => {
    // spec(W1-C-b) — the Mock defaults the FULL drive set (send_message/pause/resume) ON so Mock-driven
    // drive-control flows exercise the enabled path (production UdsGatewayPort defaults to an EMPTY set).
    const set = new MockGatewayPort().enabledSessionControls;
    expect(set.has("session.send_message")).toBe(true);
    expect(set.has("session.pause")).toBe(true);
    expect(set.has("session.resume")).toBe(true);
  });

  it("mock_get_execution_profiles_includes_two_credentialed_for_the_picker", async () => {
    // spec(W1-C-a) — the picker needs ≥2 CREDENTIALED profiles so a "change to a DIFFERENT profile" flow
    // is exercisable (one is_default + one non-default credentialed) alongside ≥1 needs-credential row.
    const result = await new MockGatewayPort().get_execution_profiles();
    const credentialed = result.profiles.filter((p) => p.has_credential);
    expect(credentialed.length).toBeGreaterThanOrEqual(2);
    expect(credentialed.some((p) => p.is_default)).toBe(true);
    expect(credentialed.some((p) => !p.is_default)).toBe(true);
  });

  it("mock_get_execution_profiles_returns_canned", async () => {
    // spec(W1-prof) — the Mock returns ≥1 canned profile (one is_default:true for the picker's
    // default-preselect; one has_credential:false for the "needs credential" affordance), validated
    // THROUGH the frozen ProfileRow shadow so it can never emit a profile the real boundary would reject.
    // The Mock validates its canned data THROUGH GetExecutionProfilesResult.parse internally (z.array(
    // ProfileRow)), so every returned row is boundary-valid by construction — assert the picker-relevant
    // shape (default-preselect + needs-credential), not a redundant per-row re-parse.
    const result = await new MockGatewayPort().get_execution_profiles();
    expect(result.profiles.length).toBeGreaterThanOrEqual(1);
    expect(result.profiles.some((p) => p.is_default)).toBe(true);
    expect(result.profiles.some((p) => !p.has_credential)).toBe(true);
  });
});
