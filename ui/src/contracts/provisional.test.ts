import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import {
  ApprovalQueueRow,
  DiffLine,
  DiffResult,
  GetDiffParams,
  GetExecutionProfilesResult,
  Hunk,
  ProfileRow,
  ProjectActivityRow,
  PullRequestRow,
  ReviewRow,
  ServerFrame,
  SessionRow,
  TerminalOutputFrame,
  WireError,
} from "./index";

// Read the FROZEN schema at test time — the §2.5-seam schema-snapshot for the
// provisional ServerFrame frame-mux (ARCHITECTURE.md §6.4). A daemon frame-shape
// change (a property added/removed on either variant) fails this loudly, the same
// way the generated-enum drift test fails on a value-set change.
const schemaPath = fileURLToPath(
  new URL(
    "../../../shared/contracts/schema/nexusops-contract.schema.json",
    import.meta.url,
  ),
);

describe("provisional ServerFrame (§6.4 frame-mux)", () => {
  it("serverframe_variant_fields_match_frozen_schema", () => {
    // spec(§6.4) — §2.5-seam: the provisional ServerFrame's two variant field-sets
    // must equal the frozen schema's ServerFrame.oneOf variant property sets,
    // keyed by the `frame_type` discriminant (rpc_response / subscription_push).
    const schema = JSON.parse(readFileSync(schemaPath, "utf8")) as {
      $defs: Record<
        string,
        {
          oneOf?: { properties: Record<string, { const?: string }> }[];
          properties?: Record<string, unknown>;
        }
      >;
    };
    const frozen = schema.$defs.ServerFrame!.oneOf!;
    const frozenByTag = new Map(
      frozen.map((v) => [
        v.properties.frame_type!.const!,
        Object.keys(v.properties).toSorted(),
      ]),
    );

    for (const variant of ServerFrame.options) {
      const tag = variant.shape.frame_type.value;
      const provisionalKeys = Object.keys(variant.shape).toSorted();
      expect(
        frozenByTag.get(tag),
        `provisional ServerFrame has a tag "${tag}" absent from the frozen schema`,
      ).toBeDefined();
      expect(provisionalKeys, `field drift in ServerFrame:${tag}`).toEqual(
        frozenByTag.get(tag),
      );
    }

    // Both directions: every frozen variant is present in the provisional union.
    expect(
      ServerFrame.options.map((v) => v.shape.frame_type.value).toSorted(),
    ).toEqual([...frozenByTag.keys()].toSorted());

    // WireError is a frozen §6.4 $def hand-modeled here (reachable via
    // rpc_response.error) — pin its field-set both directions too (== ["code"]).
    expect(Object.keys(WireError.shape).toSorted()).toEqual(
      Object.keys(schema.$defs.WireError!.properties!).toSorted(),
    );
  });

  it("serverframe_variant_id_type_matches_frozen_schema", () => {
    // spec(§6.4) — the field-set snapshot above pins NAMES, not TYPES. The two
    // variants type `id` DIFFERENTLY in the frozen schema: `rpc_response.id` is a
    // REQUIRED uint64 INTEGER (the RPC correlation id), while `subscription_push.id`
    // is a nullable + optional (id ∉ required) string. A numeric correlation id
    // modeled as `z.string()` silently passes the field-name snapshot but would
    // REJECT a real RPC frame at `.parse()` — pin the divergence behaviorally so a
    // type regression on the discriminant-adjacent `id` fails loudly.
    expect(
      ServerFrame.safeParse({ frame_type: "rpc_response", id: 7 }).success,
      "rpc_response.id is a uint64 integer (frozen schema) — numeric id must parse",
    ).toBe(true);
    expect(
      ServerFrame.safeParse({ frame_type: "rpc_response", id: "7" }).success,
      "rpc_response.id is an integer, not a string — a string id must be rejected",
    ).toBe(false);

    // subscription_push.id: string | null | absent (id ∉ required) all parse.
    const sub = { frame_type: "subscription_push", projection: "Session", kind: "upsert" };
    expect(ServerFrame.safeParse({ ...sub, id: "s1" }).success).toBe(true);
    expect(ServerFrame.safeParse({ ...sub, id: null }).success).toBe(true);
    expect(ServerFrame.safeParse(sub).success).toBe(true);
  });

  it("serverframe_terminal_output_seq_type_matches_frozen_schema", () => {
    // spec(§6.4) — Lesson §14: a §2.5-seam type adopted AHEAD of its consumer
    // carries a field-TYPE pin, not just a field-name pin (the name-set snapshot
    // alone missed rpc_response.id's integer-vs-string at 040). The 0.23.0
    // `terminal_output` frame types `seq` as a frozen uint64 INTEGER (required —
    // the PTY chunk sequence); `terminal_id` (an opaque daemon handle) + `data`
    // (base64 raw bytes) stay opaque strings. Pin seq's type behaviorally so a
    // daemon seq-type change fails loudly.
    const to = { frame_type: "terminal_output", terminal_id: "t1", data: "aGk=" };
    expect(
      ServerFrame.safeParse({ ...to, seq: 0 }).success,
      "terminal_output.seq is a uint64 integer (frozen schema) — a numeric seq must parse",
    ).toBe(true);
    expect(ServerFrame.safeParse({ ...to, seq: 42 }).success).toBe(true);
    expect(
      ServerFrame.safeParse({ ...to, seq: "42" }).success,
      "terminal_output.seq is an integer, not a string — a string seq must be rejected",
    ).toBe(false);
    // The frozen schema also constrains seq with `minimum: 0` (uint64) — pin the
    // .nonnegative() bound so a future drop of it fails loudly (Lesson §14).
    expect(
      ServerFrame.safeParse({ ...to, seq: -1 }).success,
      "terminal_output.seq is a uint64 (minimum 0) — a negative seq must be rejected",
    ).toBe(false);
  });
});

// (ui-065: the metricquality_provisional_matches_frozen_schema drift-pin retired — MetricQuality is now a
//  generated enum, pinned by the §5.0 generated drift gate `generated_zod_member_sets_equal_frozen_schema`.)

describe("provisional diff shapes (§6.1 — the 6.3e get_diff read surface)", () => {
  it("diff_shapes_field_sets_match_frozen_schema", () => {
    // spec(§6.1) — §2.5-seam: the 4 new diff object shapes the ui hand-models as
    // provisional shadows (DiffResult/Hunk/DiffLine/GetDiffParams) must equal the
    // frozen schema $defs' property sets, both directions. A daemon diff-shape change
    // (a field added/removed) fails this loudly, the same way the generated-enum drift
    // test fails on a value-set change (Lesson §2/§14).
    const schema = JSON.parse(readFileSync(schemaPath, "utf8")) as {
      $defs: Record<string, { properties?: Record<string, unknown> }>;
    };
    const cases: [string, { shape: Record<string, unknown> }][] = [
      ["DiffResult", DiffResult],
      ["Hunk", Hunk],
      ["DiffLine", DiffLine],
      ["GetDiffParams", GetDiffParams],
    ];
    for (const [name, shadow] of cases) {
      const frozen = Object.keys(schema.$defs[name]!.properties!).toSorted();
      expect(
        Object.keys(shadow.shape).toSorted(),
        `field drift in ${name}`,
      ).toEqual(frozen);
    }
  });

  it("hunk_offsets_are_uint32", () => {
    // spec(§6.1) — Lesson §14: a §2.5-seam type adopted AHEAD of its consumer carries
    // a field-TYPE pin, not just a field-name pin. The frozen Hunk offsets
    // (old_start/old_lines/new_start/new_lines) are uint32 INTEGERS (minimum 0); a
    // string-typed offset would pass the name-set snapshot but reject a real frame.
    const base = {
      header: "@@ -1,2 +1,3 @@",
      old_start: 1,
      old_lines: 2,
      new_start: 1,
      new_lines: 3,
      lines: [{ kind: "context", content: " a\n" }],
    };
    expect(Hunk.safeParse(base).success, "numeric uint32 offsets must parse").toBe(true);
    for (const field of ["old_start", "old_lines", "new_start", "new_lines"]) {
      expect(
        Hunk.safeParse({ ...base, [field]: "1" }).success,
        `${field} is a uint32 integer, not a string — a string offset must be rejected`,
      ).toBe(false);
      expect(
        Hunk.safeParse({ ...base, [field]: -1 }).success,
        `${field} is a uint32 (minimum 0) — a negative offset must be rejected`,
      ).toBe(false);
    }
  });

  it("profile_row_field_sets_match_frozen_schema", () => {
    // spec(§6.1/§2.5-seam W1-prof) — ProfileRow + GetExecutionProfilesResult are hand-modeled provisional
    // shadows of the frozen read-RPC RESULT $defs (objects aren't generated — Lesson §2; the DiffResult
    // precedent). The shadow field-sets must equal the frozen schema $defs' property sets (drift loudly
    // on a daemon ProfileRow change).
    const schema = JSON.parse(readFileSync(schemaPath, "utf8")) as {
      $defs: Record<string, { properties?: Record<string, unknown> }>;
    };
    const cases: [string, { shape: Record<string, unknown> }][] = [
      ["ProfileRow", ProfileRow],
      ["GetExecutionProfilesResult", GetExecutionProfilesResult],
    ];
    for (const [name, shadow] of cases) {
      const frozen = Object.keys(schema.$defs[name]!.properties!).toSorted();
      expect(
        Object.keys(shadow.shape).toSorted(),
        `field drift in ${name}`,
      ).toEqual(frozen);
    }
  });

  it("profile_row_status_delegates_to_generated_executionprofile_enum", () => {
    // spec(§6.1/§5.1) — ProfileRow.status delegates to the GENERATED ExecutionProfile enum (never a
    // re-literal'd union, Lesson §1/§2): a canonical state parses, an unknown state is rejected.
    const base = {
      execution_profile_id: "ep_1",
      provider: "anthropic",
      harness: "claude_code",
      model: null,
      account_alias: null,
      status: "available",
      is_default: true,
      has_credential: false,
    };
    expect(ProfileRow.safeParse(base).success).toBe(true);
    expect(ProfileRow.safeParse({ ...base, status: "credit_exhausted" }).success).toBe(true);
    expect(ProfileRow.safeParse({ ...base, status: "bogus_state" }).success).toBe(false);
  });

  it("profile_row_is_secret_free_rejects_keychain_ref_and_unknown_fields", () => {
    // spec(§15 #4) — the keychain POINTER / secret is NEVER served: ProfileRow has NO keychain_ref field;
    // the .strict() shadow REJECTS a leaked keychain_ref (or any extra field) — the secret-free pin
    // mirrored UI-side (the daemon's deny_unknown_fields adversarial pin, W1-prof seal).
    const base = {
      execution_profile_id: "ep_1",
      provider: "anthropic",
      harness: "claude_code",
      model: null,
      account_alias: null,
      status: "available",
      is_default: true,
      has_credential: false,
    };
    expect(ProfileRow.safeParse({ ...base, keychain_ref: "kc://leak" }).success).toBe(false);
    expect(ProfileRow.safeParse({ ...base, extra: 1 }).success).toBe(false);
    expect(GetExecutionProfilesResult.safeParse({ profiles: [], extra: 1 }).success).toBe(false);
  });

  it("profile_row_field_types_reject_wrong_primitives", () => {
    // spec(LESSON §14) — a field-TYPE pin in the NEGATIVE direction (the hunk_offsets_are_uint32
    // precedent): a wrong-typed value must be REJECTED, not just the right one accepted. A shadow that
    // accidentally declared z.string()/z.number() for the booleans (or vice-versa) would pass the
    // field-name drift test + the positive tests yet reject real daemon frames.
    const base = {
      execution_profile_id: "ep_1",
      provider: "anthropic",
      harness: "claude_code",
      model: null,
      account_alias: null,
      status: "available",
      is_default: true,
      has_credential: false,
    };
    expect(ProfileRow.safeParse(base).success).toBe(true);
    // booleans are STRICT booleans (no string/number coercion)
    expect(ProfileRow.safeParse({ ...base, is_default: "true" }).success).toBe(false);
    expect(ProfileRow.safeParse({ ...base, is_default: 1 }).success).toBe(false);
    expect(ProfileRow.safeParse({ ...base, has_credential: "false" }).success).toBe(false);
    expect(ProfileRow.safeParse({ ...base, has_credential: 0 }).success).toBe(false);
    // string fields reject a non-string
    expect(ProfileRow.safeParse({ ...base, execution_profile_id: 123 }).success).toBe(false);
  });

  it("profile_row_optionals_accept_null_or_string", () => {
    // spec(LESSON §15 trap 3) — model/account_alias serialize as explicit null (no skip_serializing_if);
    // the shadow accepts null AND a string (and tolerates omission — they're not in `required`).
    const base = {
      execution_profile_id: "ep_1",
      provider: "anthropic",
      harness: "claude_code",
      status: "available",
      is_default: false,
      has_credential: true,
    };
    expect(ProfileRow.safeParse({ ...base, model: null, account_alias: null }).success).toBe(true);
    expect(
      ProfileRow.safeParse({ ...base, model: "claude-x", account_alias: "work" }).success,
    ).toBe(true);
  });

  it("diff_line_kind_delegates_to_generated_enum", () => {
    // spec(§6.1) — DiffLine.kind delegates to the GENERATED DiffLineKind validator
    // (never a re-literal'd union, Lesson §1/§2): every canonical kind parses, an
    // unknown kind is rejected (reject-unknown end-to-end, §5.0/§15).
    expect(DiffLine.safeParse({ kind: "context", content: " x\n" }).success).toBe(true);
    expect(DiffLine.safeParse({ kind: "added", content: "+x\n" }).success).toBe(true);
    expect(DiffLine.safeParse({ kind: "removed", content: "-x\n" }).success).toBe(true);
    expect(DiffLine.safeParse({ kind: "bogus", content: "x\n" }).success).toBe(false);
  });

  it("diff_shapes_reject_unknown_fields", () => {
    // spec(§6.1) — the frozen diff $defs are `additionalProperties:false`; the
    // shadows are `.strict()` to match. The field-set snapshot pins NAMES but would
    // NOT catch an accidental `.strict()` drop — so pin the reject-extra behavior
    // directly: a well-formed value with one extra field must FAIL each shadow.
    expect(
      DiffResult.safeParse({ hunks: [], extra: 1 }).success,
      "DiffResult must reject an unknown field (.strict)",
    ).toBe(false);
    expect(
      DiffLine.safeParse({ kind: "added", content: "+x\n", extra: 1 }).success,
      "DiffLine must reject an unknown field (.strict)",
    ).toBe(false);
    expect(
      GetDiffParams.safeParse({ worktree_id: "wt_1", file: "a.ts", extra: 1 }).success,
      "GetDiffParams must reject an unknown field (.strict)",
    ).toBe(false);
    expect(
      Hunk.safeParse({
        header: "@@ -1 +1 @@",
        old_start: 1,
        old_lines: 1,
        new_start: 1,
        new_lines: 1,
        lines: [],
        extra: 1,
      }).success,
      "Hunk must reject an unknown field (.strict)",
    ).toBe(false);
  });
});

describe("provisional terminal shadow (§6.4 — the 6.3d well consumes this)", () => {
  it("terminal_output_frame_shadow_matches_frozen_serverframe_variant", () => {
    // spec(§6.4) — the exported TerminalOutputFrame (the consumer's input shape) IS
    // the frozen ServerFrame.terminal_output variant, extracted for reuse. Field-set
    // drift-pinned both directions against the frozen schema (Lesson §2/§14).
    const schema = JSON.parse(readFileSync(schemaPath, "utf8")) as {
      $defs: Record<
        string,
        { oneOf?: { properties: Record<string, { const?: string }> }[] }
      >;
    };
    const frozenVariant = schema.$defs.ServerFrame!.oneOf!.find(
      (v) => v.properties.frame_type!.const === "terminal_output",
    )!;
    expect(Object.keys(TerminalOutputFrame.shape).toSorted()).toEqual(
      Object.keys(frozenVariant.properties).toSorted(),
    );
  });
});

describe("053 L2-prep — ApprovalQueueRow frozen-shadow + survival drift-pins (§5.0)", () => {
  const readSchema = () =>
    JSON.parse(readFileSync(schemaPath, "utf8")) as {
      $defs: Record<
        string,
        { properties?: Record<string, unknown>; oneOf?: { const?: string }[] }
      >;
    };

  it("approval_queue_row_field_set_matches_frozen_schema", () => {
    // spec(§5.0) — the FIRST frozen projection-row: the 14-field set is snapshot-pinned to the
    // frozen schema `$defs.ApprovalQueueRow` (a daemon field add/remove/rename fails this loudly,
    // the §2.5-seam shared-contract snapshot — the ServerFrame precedent).
    const schema = readSchema();
    expect(Object.keys(ApprovalQueueRow.shape).toSorted()).toEqual(
      Object.keys(schema.$defs.ApprovalQueueRow!.properties!).toSorted(),
    );
  });

  it("approval_queue_row_strict_rejects_extra_and_requires_core", () => {
    // `.strict()` per the frozen `deny_unknown_fields`; the required core present; optionals nullable.
    const base = {
      approval_id: "appr_1",
      action_request_id: null,
      plan_id: null,
      project_id: null,
      session_id: null,
      agent_team_id: null,
      risk_level: 2,
      status: "awaiting_approval",
      requester_type: "agent_session",
      requester_id: "a1",
      preview_summary: null,
      requested_at: "2026-06-14T00:00:00Z",
      expires_at: null,
      policy_decision: null,
    };
    expect(ApprovalQueueRow.safeParse(base).success).toBe(true);
    // an extra field → rejected (.strict()).
    expect(ApprovalQueueRow.safeParse({ ...base, bogus: 1 }).success).toBe(false);
    // a missing required core field → rejected.
    const noRisk: Record<string, unknown> = { ...base };
    delete noRisk.risk_level;
    expect(ApprovalQueueRow.safeParse(noRisk).success).toBe(false);
  });

  // (ui-065: the resume_mode/recovery_state drift-pins retired — ResumeMode/RecoveryState are now
  //  generated enums, pinned by the §5.0 generated drift gate.)
});

describe("ui-061 — PR + Review frozen-shadow reconcile (§5.0/§11.2)", () => {
  const readSchema = () =>
    JSON.parse(readFileSync(schemaPath, "utf8")) as {
      $defs: Record<string, { properties?: Record<string, unknown> }>;
    };

  // A frozen-shaped PullRequestRow (all 11 fields; optionals explicit-null per the daemon serve).
  const prBase = {
    pr_id: "repo_1#101",
    project_id: null,
    repo_id: null,
    pr_number: 101,
    title: null,
    status: "open",
    head_branch: null,
    base_branch: null,
    pr_checked_at: null,
    mergeable: null,
    checks_summary: null,
  };

  // A frozen-shaped ReviewRow (all 8 fields).
  const reviewBase = {
    review_id: 9001,
    pr_number: 101,
    project_id: null,
    repo_id: null,
    reviewer: null,
    state: "approved",
    submitted_at: null,
    body: null,
  };

  it("pull_request_row_field_set_matches_frozen_schema", () => {
    // spec(§11.2) — the 2nd frozen projection-row reconciled 4→11: the shadow's field-set is
    // snapshot-pinned to the frozen schema `$defs.PullRequestRow` (a daemon field add/remove/rename
    // fails this loudly, the §2.5-seam shared-contract snapshot — the ApprovalQueueRow precedent §37).
    const schema = readSchema();
    expect(Object.keys(PullRequestRow.shape).toSorted()).toEqual(
      Object.keys(schema.$defs.PullRequestRow!.properties!).toSorted(),
    );
  });

  it("pull_request_row_pr_number_is_uint_and_strict", () => {
    // spec(§11.2) — pr_number is a u64 NUMBER (the work-order str→number drift), NOT a string;
    // `.strict()` per the frozen `deny_unknown_fields`; pr_id (PK) + status are the required core.
    expect(PullRequestRow.safeParse(prBase).success).toBe(true);
    // a STRING pr_number is rejected (the drift fixed — inverted from the old z.string()).
    expect(PullRequestRow.safeParse({ ...prBase, pr_number: "101" }).success).toBe(false);
    // a negative pr_number is rejected (u64, minimum 0).
    expect(PullRequestRow.safeParse({ ...prBase, pr_number: -1 }).success).toBe(false);
    // an extra field is rejected (.strict()).
    expect(PullRequestRow.safeParse({ ...prBase, bogus: 1 }).success).toBe(false);
    // a missing required core (pr_id) is rejected.
    const noId: Record<string, unknown> = { ...prBase };
    delete noId.pr_id;
    expect(PullRequestRow.safeParse(noId).success).toBe(false);
  });

  it("pull_request_row_diff_stats_are_uint_nullable", () => {
    // spec(§11.2) — the 4 D6 diff-stat fields (additions/deletions/changed_files/commits) are u64
    // NUMBERS (frozen `Option<u64>`, format uint64, minimum 0), each nullable + optional (NULL on a
    // pre-D6/unsynced row); mirrors the pr_number uint pin. A string or negative is rejected.
    expect(
      PullRequestRow.safeParse({ ...prBase, additions: 40, deletions: 7, changed_files: 3, commits: 2 })
        .success,
    ).toBe(true);
    // a present 0 is a VALID u64 (minimum 0) — the exact value LESSON §32 protects from a falsy guard.
    expect(
      PullRequestRow.safeParse({ ...prBase, additions: 0, deletions: 0, changed_files: 0, commits: 0 })
        .success,
    ).toBe(true);
    // null / absent are tolerated (the brief: NULL where GitHub omitted them / pre-D6 rows).
    expect(
      PullRequestRow.safeParse({
        ...prBase,
        additions: null,
        deletions: null,
        changed_files: null,
        commits: null,
      }).success,
    ).toBe(true);
    // a STRING diff-stat is rejected (uint NUMBER, not a string).
    expect(PullRequestRow.safeParse({ ...prBase, additions: "40" }).success).toBe(false);
    // a NEGATIVE diff-stat is rejected (u64, minimum 0).
    expect(PullRequestRow.safeParse({ ...prBase, deletions: -1 }).success).toBe(false);
  });

  it("pull_request_row_head_sha_is_string_nullable", () => {
    // spec(§11.2/§5.0) — head_sha (P4.7) is a TEXT field (frozen `Option<String>`, direct passthrough —
    // NOT a uint like the diff-stats): a string parses; null/absent tolerated; a non-string rejects. The
    // UI head_sha is display/pin-FORMATION only (the daemon's anti-race is the LIVE GitHub 409).
    expect(PullRequestRow.safeParse({ ...prBase, head_sha: "abc123def" }).success).toBe(true);
    expect(PullRequestRow.safeParse({ ...prBase, head_sha: null }).success).toBe(true);
    // a non-string head_sha is rejected (TEXT, not a number).
    expect(PullRequestRow.safeParse({ ...prBase, head_sha: 123 }).success).toBe(false);
  });

  it("review_row_field_set_matches_frozen_schema", () => {
    // spec(§11.2) — the NEW (4th) frozen projection-row: the 8-field shadow is snapshot-pinned to
    // the frozen schema `$defs.ReviewRow` (the D5b-1 review vertical; the ApprovalQueueRow precedent).
    const schema = readSchema();
    expect(Object.keys(ReviewRow.shape).toSorted()).toEqual(
      Object.keys(schema.$defs.ReviewRow!.properties!).toSorted(),
    );
  });

  it("review_row_uint_state_delegate_and_strict", () => {
    // spec(§5.1/§11.2) — review_id/pr_number are u64 numbers; `state` delegates to the generated
    // ReviewState VALUE enum (reject-unknown); `.strict()`; review_id (PK) + state are required core.
    expect(ReviewRow.safeParse(reviewBase).success).toBe(true);
    // review_id is a uint NUMBER — a string is rejected.
    expect(ReviewRow.safeParse({ ...reviewBase, review_id: "9001" }).success).toBe(false);
    // state delegates to the generated ReviewState — an unknown verdict is rejected (not a loose string).
    expect(ReviewRow.safeParse({ ...reviewBase, state: "bogus" }).success).toBe(false);
    // an extra field is rejected (.strict()).
    expect(ReviewRow.safeParse({ ...reviewBase, bogus: 1 }).success).toBe(false);
    // a missing required core (review_id) is rejected.
    const noId: Record<string, unknown> = { ...reviewBase };
    delete noId.review_id;
    expect(ReviewRow.safeParse(noId).success).toBe(false);
  });
});

describe("ui-062 — SessionRow recovery-shadow reconcile (§5.1/§11.4)", () => {
  const readSchema = () =>
    JSON.parse(readFileSync(schemaPath, "utf8")) as {
      $defs: Record<string, { properties?: Record<string, unknown> }>;
    };

  // A frozen-shaped SessionRow (all 10 fields; optionals explicit-null per the daemon serve).
  const sessionBase = {
    session_id: "sess_1",
    project_id: "project_1",
    status: "active",
    display_name: null,
    harness: null,
    model: null,
    execution_profile_id: null,
    resume_mode: null,
    replayed_event_count: null,
    recovered_at: null,
  };

  it("session_row_field_set_matches_frozen_schema", () => {
    // spec(§5.1/§11.4) — the SessionRow shadow reconciled 5→10: the field-set is snapshot-pinned to
    // the frozen schema `$defs.SessionRow` (the survival/recovery fields now have a consumer — the
    // resume-mode indicator; the ApprovalQueueRow precedent §37). `title` → `display_name`.
    const schema = readSchema();
    expect(Object.keys(SessionRow.shape).toSorted()).toEqual(
      Object.keys(schema.$defs.SessionRow!.properties!).toSorted(),
    );
  });

  it("session_row_recovery_fields_strict_and_uint", () => {
    // spec(§11.4) — `project_id` is required (non-Option — the daemon NOT-NULL); `display_name` (not
    // `title`); the survival fields typed: `replayed_event_count` uint, `recovered_at` string,
    // `resume_mode` delegates to the ResumeMode shadow; `.strict()` per the frozen `deny_unknown_fields`.
    expect(SessionRow.safeParse(sessionBase).success).toBe(true);
    // a real recovered session parses (resume_mode + recovery fields set).
    expect(
      SessionRow.safeParse({
        ...sessionBase,
        display_name: "Refactor auth",
        resume_mode: "replayed",
        replayed_event_count: 128,
        recovered_at: "2026-06-16T08:30:00Z",
      }).success,
    ).toBe(true);
    // project_id is REQUIRED (non-Option) — a missing project_id is rejected.
    const noProject: Record<string, unknown> = { ...sessionBase };
    delete noProject.project_id;
    expect(SessionRow.safeParse(noProject).success).toBe(false);
    // replayed_event_count is a uint NUMBER — a string is rejected.
    expect(SessionRow.safeParse({ ...sessionBase, replayed_event_count: "128" }).success).toBe(false);
    // resume_mode delegates to the ResumeMode shadow — an unknown mode is rejected.
    expect(SessionRow.safeParse({ ...sessionBase, resume_mode: "bogus" }).success).toBe(false);
    // `.strict()` — `title` is no longer a field (renamed to display_name) → an extra field rejected.
    expect(SessionRow.safeParse({ ...sessionBase, title: "x" }).success).toBe(false);
  });
});

describe("ProjectActivityRow (provisional — the daemon's name-less counters shape)", () => {
  it("projectactivity_row_parses_without_name", () => {
    // spec(§7.2) — the daemon serves proj_project_activity (a COUNTERS table, NO `name` col, loose
    // serve) → a real bare row must parse: `name` optional + extra counter cols tolerated (NOT .strict).
    const row = ProjectActivityRow.parse({
      project_id: "proj_x",
      active_sessions: 2,
      open_prs: 0,
      updated_at_seq: 5,
    });
    expect(row.project_id).toBe("proj_x");
    expect(row.name).toBeUndefined();
  });

  it("projectactivity_row_still_accepts_a_name", () => {
    // forward-compat: a row WITH a name (the deferred daemon friendly-name) still binds.
    expect(ProjectActivityRow.parse({ project_id: "proj_x", name: "auth" }).name).toBe("auth");
  });

  it("projectactivity_row_requires_project_id", () => {
    expect(ProjectActivityRow.safeParse({ name: "auth" }).success).toBe(false);
  });
});
