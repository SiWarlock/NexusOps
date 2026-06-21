import { describe, it, expect } from "vitest";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { PR_MUTATION_ACTION_TYPES, buildMergePrActionRequest } from "./merge-pr-request";

// ui-070 cat-1 — the PR-mutation type set + the held-flip discipline + the merge_pr intent formation
// (ruling A: the UI sends inputs:{pr_number,sha,merge_method} + a repo resource_ref, and NEVER names
// the GitHub owner/repo — the daemon resolves them from repo_id, the D7/§59 pattern).

describe("buildMergePrActionRequest (cat-1 ui-070, ruling A)", () => {
  it("build_merge_pr_action_request_shape", () => {
    // spec(§6.2 / ruling A) — a typed ActionRequest (mirror buildHunkActionRequest): action_type
    // github.merge_pr, resource_refs:[{type:"repo",id}], inputs:{pr_number,sha,merge_method:"merge"},
    // requester user/current_user, daemon-minted (empty) id, created_at = submit time. The UI NEVER
    // names owner/repo (ruling A — the daemon resolves them from repo_id).
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
    // ruling A: the UI never names the GitHub owner/repo (the daemon resolves from repo_id).
    expect(req.inputs).not.toHaveProperty("owner");
    expect(req.inputs).not.toHaveProperty("repo");
    expect(req.created_at).toBe("2026-06-21T00:00:00Z");
    // risk_level is a NON-AUTHORITATIVE hint (daemon-reconciled to catalog risk-3, never displayed) —
    // pinned at 0 to mirror buildHunkActionRequest, so a change can't silently make it look authoritative.
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

describe("PR_MUTATION_ACTION_TYPES (ui-070)", () => {
  it("pr_mutation_action_types_contains_merge_pr", () => {
    // spec(§6.2/ui-070) — github.merge_pr is the (first) PR-mutation type gated behind the separate
    // prMutationsEnabled flag (extensible — github.submit_review joins as its own later cat-1 slice).
    expect(PR_MUTATION_ACTION_TYPES.has("github.merge_pr")).toBe(true);
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

describe("ui-070 cat-1 — prMutationsEnabled held-flip discipline", () => {
  it("production_construction_never_sets_pr_mutations_enabled", () => {
    // spec(cat-1 [[27]]) — NO production (non-test) source sets prMutationsEnabled to a literal `true`:
    // the go-live is a future USER-signed-off slice, so today NO production path reaches a live PR
    // mutation (the provably-unreachable layer; this guard goes RED the instant someone flips it).
    const offenders = walk("src").filter((f) =>
      /prMutationsEnabled\s*:\s*true/.test(readFileSync(f, "utf8")),
    );
    expect(offenders).toEqual([]);
  });
});
