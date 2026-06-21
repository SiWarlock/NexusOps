import { describe, it, expect } from "vitest";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import {
  PR_MUTATION_ACTION_TYPES,
  buildMergePrActionRequest,
  buildSubmitReviewActionRequest,
  isPrMutationEnabled,
} from "./pr-mutation-request";

// ui-070/071 cat-1 — the PR-mutation family: the shared action-type set + the per-action enablement gate
// (ui-071 fork-1b) + the merge_pr (ui-070) and submit_review (ui-071) intent builders + the held-flip pin.

describe("PR_MUTATION_ACTION_TYPES (ui-070/071)", () => {
  it("pr_mutation_action_types_contains_both", () => {
    // spec(§6.2/ui-071) — the gate set covers BOTH github writes (merge_pr + submit_review); the
    // per-action gate + the port guard key off this set.
    expect([...PR_MUTATION_ACTION_TYPES].toSorted()).toEqual([
      "github.merge_pr",
      "github.submit_review",
    ]);
  });
});

describe("isPrMutationEnabled (ui-071 per-action gate, fork-1b)", () => {
  it("is_pr_mutation_enabled_per_action", () => {
    // spec(ui-071/1b) — true only for an action_type IN the enabledPrMutations set (per-action, so the
    // future go-live can stage lowest-risk-first); a non-member is false even if the set is non-empty.
    const port = { enabledPrMutations: new Set(["github.submit_review"]) };
    expect(isPrMutationEnabled(port, "github.submit_review")).toBe(true);
    expect(isPrMutationEnabled(port, "github.merge_pr")).toBe(false);
    expect(isPrMutationEnabled({ enabledPrMutations: new Set<string>() }, "github.merge_pr")).toBe(
      false,
    );
  });
});

describe("buildMergePrActionRequest (cat-1 ui-070, ruling A)", () => {
  it("build_merge_pr_action_request_shape", () => {
    // spec(§6.2 / ruling A) — a typed ActionRequest: action_type github.merge_pr,
    // resource_refs:[{type:"repo",id}], inputs:{pr_number,sha,merge_method:"merge"}, requester
    // user/current_user, daemon-minted (empty) id. The UI NEVER names owner/repo (daemon resolves from repo_id).
    const req = buildMergePrActionRequest(
      { repo_id: "repo_1", pr_number: 101, head_sha: "abc123", merge_method: "merge" },
      "2026-06-21T00:00:00Z",
    );
    expect(req.action_type).toBe("github.merge_pr");
    expect(req.action_request_id).toBe(""); // daemon mints
    expect(req.requester_type).toBe("user");
    expect(req.requester_id).toBe("current_user");
    expect(req.resource_refs).toEqual([{ type: "repo", id: "repo_1" }]);
    expect(req.inputs).toEqual({ pr_number: 101, sha: "abc123", merge_method: "merge" });
    expect(req.inputs).not.toHaveProperty("owner");
    expect(req.inputs).not.toHaveProperty("repo");
    expect(req.created_at).toBe("2026-06-21T00:00:00Z");
    // risk_level is a NON-AUTHORITATIVE hint (daemon-reconciled to catalog risk-3, never displayed).
    expect(req.risk_level).toBe(0);
  });

  it("merge_pr_intent_pins_displayed_head_sha", () => {
    // spec([[19]]/D2) — inputs.sha == the DISPLAYED head_sha passed in (submitted==displayed; the
    // anti-race pin the daemon 409s against), NOT re-fetched or derived.
    const req = buildMergePrActionRequest(
      { repo_id: "repo_1", pr_number: 7, head_sha: "deadbeefsha", merge_method: "merge" },
      "2026-06-21T00:00:00Z",
    );
    expect((req.inputs as { sha: string }).sha).toBe("deadbeefsha");
  });
});

describe("buildSubmitReviewActionRequest (cat-1 ui-071, ruling A / fork-2b)", () => {
  it("build_submit_review_action_request_shape", () => {
    // spec(§6.2 / ruling A / daemon LESSON 61) — action_type github.submit_review,
    // resource_refs:[{type:"repo",id}], inputs:{pr_number, commit_id, event, body}, requester
    // user/current_user, daemon-minted id; the UI NEVER names owner/repo (daemon resolves from repo_id).
    const req = buildSubmitReviewActionRequest(
      {
        repo_id: "repo_1",
        pr_number: 101,
        head_sha: "abc123",
        event: "request_changes",
        body: "needs work",
      },
      "2026-06-21T00:00:00Z",
    );
    expect(req.action_type).toBe("github.submit_review");
    expect(req.action_request_id).toBe("");
    expect(req.requester_type).toBe("user");
    expect(req.requester_id).toBe("current_user");
    expect(req.resource_refs).toEqual([{ type: "repo", id: "repo_1" }]);
    expect(req.inputs).toEqual({
      pr_number: 101,
      commit_id: "abc123",
      event: "request_changes",
      body: "needs work",
    });
    expect(req.inputs).not.toHaveProperty("owner");
    expect(req.inputs).not.toHaveProperty("repo");
    expect(req.risk_level).toBe(0); // non-authoritative hint (daemon catalog risk-3)
    expect(req.created_at).toBe("2026-06-21T00:00:00Z");
  });

  it("submit_review_pins_displayed_head_sha_as_commit_id", () => {
    // spec([[19]]/D2) — inputs.commit_id == the displayed head_sha (the reviewed head the daemon 422/409s
    // against), NOT re-fetched.
    const req = buildSubmitReviewActionRequest(
      { repo_id: "repo_1", pr_number: 7, head_sha: "reviewedsha", event: "approve", body: "" },
      "2026-06-21T00:00:00Z",
    );
    expect((req.inputs as { commit_id: string }).commit_id).toBe("reviewedsha");
  });

  it("submit_review_body_trimmed", () => {
    // spec([[33]]) — body is .trim()'d before inclusion (a whitespace-only body → "" for approve; the
    // trimmed text for the verdicts) so a whitespace body can't become the audited review content.
    const approve = buildSubmitReviewActionRequest(
      { repo_id: "r", pr_number: 1, head_sha: "s", event: "approve", body: "   " },
      "2026-06-21T00:00:00Z",
    );
    expect((approve.inputs as { body: string }).body).toBe(""); // whitespace → empty for approve
    const comment = buildSubmitReviewActionRequest(
      { repo_id: "r", pr_number: 1, head_sha: "s", event: "comment", body: "  looks fine  " },
      "2026-06-21T00:00:00Z",
    );
    expect((comment.inputs as { body: string }).body).toBe("looks fine"); // trimmed text
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

describe("ui-070/071 cat-1 — enabledPrMutations held-flip discipline", () => {
  it("production_construction_never_enables_pr_mutations", () => {
    // spec(cat-1 [[27]]/[[28]]) — NO production (non-test) source constructs a port with a NON-EMPTY
    // enabledPrMutations VALUE (the go-live flip is a future USER-signed-off slice → today NO production
    // path reaches a live PR mutation). The scan flags any `enabledPrMutations:` VALUE that is neither the
    // empty `new Set()` nor a `ReadonlySet<…>` TYPE annotation (the field/option decls) — i.e. a populated
    // construction (`new Set([...])`, `PR_MUTATION_ACTION_TYPES`, any var) goes RED. (The Mock's default
    // full set is an `=` assignment, not a `:` value, and Mock is dev/test-only — never the production seam.)
    const offenders = walk("src").filter((f) => {
      const src = readFileSync(f, "utf8");
      // every `enabledPrMutations: <value>` occurrence; a VALUE is an object-literal construction option,
      // a TYPE is the `ReadonlySet<…>` field/param decl. Flag any value that is neither a `ReadonlySet`
      // type nor the empty `new Set()` — a populated construction (the go-live flip) goes RED.
      for (const m of src.matchAll(/enabledPrMutations\s*:\s*([^\n;,]+)/g)) {
        const value = m[1]!.trim();
        // exclude the `ReadonlySet<…>` TYPE decl + the safe empty `new Set()` construction; anything
        // else (a populated `new Set([...])`, `PR_MUTATION_ACTION_TYPES`, a var) is a flip → RED.
        if (!value.startsWith("ReadonlySet") && !/^new Set\s*<[^>]*>?\s*\(\s*\)/.test(value) && !/^new Set\s*\(\s*\)/.test(value)) {
          return true;
        }
      }
      return false;
    });
    expect(offenders).toEqual([]);
  });
});
