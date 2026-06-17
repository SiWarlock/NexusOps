// @vitest-environment jsdom
//
// Layer C integration (P6.8) — "live data stays live": a daemon ProjectionDelta keeps the cockpit
// current. Both the Session (ui-062) and ApprovalQueue (ui-059) streams consume a `row:None` id-NUDGE
// via REFETCH-ON-NUDGE (a coalesced re-read of get_projection), NOT a row-apply reducer (which no-ops
// on the absent row — LESSON §29). A dedicated fake gateway (extends MockGatewayPort for the reads)
// holds the nudge until the test captures a baseline, then serves an AUGMENTED page on the re-read so
// the live change is observable only via the refetch (proving it's not a row-apply).
import { describe, it, expect, afterEach, vi } from "vitest";
import { cleanup, render, screen, waitFor, fireEvent } from "@testing-library/react";

// xterm is a canvas lib (not jsdom-friendly) — mirror Shell.test.tsx's mock.
vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    open() {}
    write() {}
    loadAddon() {}
    dispose() {}
    onData() {
      return { dispose() {} };
    }
    resize() {}
  },
}));
vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit() {}
    activate() {}
    dispose() {}
  },
}));

import { Shell } from "./Shell";
import { MockGatewayPort } from "../gateway-client/mock";
import type {
  ApprovalQueuePage,
  ApprovalQueueRow,
  ProjectActivityPage,
  ProjectActivityRow,
  ProjectionDelta,
  ProjectionName,
  ProjectionPageByName,
  PullRequestProjectionPage,
  PullRequestRow,
  ReviewProjectionPage,
  ReviewRow,
  SessionProjectionPage,
  SessionRow,
  UsageProjectionPage,
  UsageRow,
} from "../contracts/index";
import type {
  ProjectionScope,
  ProjectionPageParams,
  SubscribeParams,
} from "../gateway-client/types";
import { makeApprovalRow } from "../projections/fixtures/proj_approval_queue";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";

/** A gateway that holds the Session `row:None` nudge until the test releases it (race-free baseline
 *  capture), then serves an AUGMENTED Session page (one extra session) on the post-nudge re-read. A
 *  row-apply reducer would no-op on the absent row → the extra session would never appear; only
 *  refetch-on-nudge surfaces it (ui-062 — the §29 pattern generalized from ApprovalQueue to Session).
 *  ApprovalQueue stays healthy via super. */
class GatedSessionRefetchGateway extends MockGatewayPort {
  sessionReads = 0;
  private nudgeReleased = false;
  private releaseNudgeFn!: () => void;
  private readonly nudgeGate: Promise<void>;
  constructor(private readonly extra: SessionRow) {
    super();
    this.nudgeGate = new Promise<void>((resolve) => {
      this.releaseNudgeFn = resolve;
    });
  }
  releaseNudge(): void {
    this.nudgeReleased = true; // the live change is now visible to subsequent re-reads
    this.releaseNudgeFn();
  }
  async *subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta> {
    if (params.projection === "Session") {
      await this.nudgeGate; // hold the nudge until the test captures the baseline
      yield { projection: "Session", kind: "upsert", id: "sess_live_nudge" }; // row:None
      await new Promise<void>(() => {
        /* stay open — a live stream; the never-resolving promise is abandoned at test teardown. */
      });
      return;
    }
    yield* super.subscribe(params); // ApprovalQueue: the mock's benign delta + stay open
  }
  async get_projection<K extends ProjectionName>(
    name: K,
    scope?: ProjectionScope,
    page?: ProjectionPageParams,
  ): Promise<ProjectionPageByName[K]> {
    const result = await super.get_projection(name, scope, page);
    if (name === "Session") {
      this.sessionReads++;
      // The augmented session appears ONLY after the nudge is released (the live change) — so ANY
      // pre-nudge read (the load + any recovery read) is the base set, regardless of count.
      if (this.nudgeReleased) {
        const s = result as SessionProjectionPage;
        return { ...s, rows: [...s.rows, this.extra] } as ProjectionPageByName[K];
      }
    }
    return result;
  }
}

afterEach(cleanup);

describe("Shell live Session subscribe (ui-062 — refetch-on-nudge)", () => {
  it("session_subscribe_refetches_on_row_none_nudge", async () => {
    // ui-062: a daemon Session delta is a `row:None` id-NUDGE (SessionStarted/Failed/Recovered) → the
    // live list must REFETCH get_projection("Session"), NOT apply the absent delta row (the removed
    // applySessionDelta no-op'd on row:None, §29). The new session can ONLY appear via the re-read.
    const extra: SessionRow = {
      session_id: "session_live_refetched",
      status: "active",
      display_name: "Live new session",
      project_id: projectActivityFixture.rows[0]!.project_id, // the default active project → renders in the tree
    };
    const gateway = new GatedSessionRefetchGateway(extra);
    const { container } = render(<Shell gateway={gateway} />);

    // load settles → the nudge is still gated → the new session is NOT present yet (NO refetch).
    await screen.findByTestId("sidebar-waiting-badge");
    expect(
      container.querySelector('[data-item-id="Session:session_live_refetched"]'),
    ).toBeNull();
    const readsBeforeNudge = gateway.sessionReads;

    gateway.releaseNudge(); // the row:None nudge fires → coalesced refetch → augmented re-read

    await waitFor(() => {
      // the nudge caused a RE-READ (not a row-apply)…
      expect(gateway.sessionReads).toBeGreaterThan(readsBeforeNudge);
      // …and the re-read's extra session is now in the live tree.
      expect(
        container.querySelector('[data-item-id="Session:session_live_refetched"]'),
      ).not.toBeNull();
    });
  });
});

// ── ui-059 — the live ApprovalQueue subscription (refetch-on-nudge + the 2nd stream's recovery) ──────
// A daemon ApprovalQueue delta is a `row:None` id-NUDGE → the live queue must RE-READ get_projection,
// NOT apply the (absent) delta row. These integration pins drive a daemon-shaped Mock through the real
// Shell 2nd-subscribe effect.

/** A gateway that holds the ApprovalQueue `row:None` nudge until the test releases it (race-free baseline
 *  capture), then serves an AUGMENTED ApprovalQueue (one extra PENDING approval) on the post-nudge
 *  re-read. A row-apply reducer would no-op on the absent row → the extra approval would never count;
 *  only refetch-on-nudge surfaces it (the waiting badge increments). Session stays healthy via super. */
class GatedApprovalRefetchGateway extends MockGatewayPort {
  approvalReads = 0;
  private nudgeReleased = false;
  private releaseNudgeFn!: () => void;
  private readonly nudgeGate: Promise<void>;
  constructor(private readonly extra: ApprovalQueueRow) {
    super();
    this.nudgeGate = new Promise<void>((resolve) => {
      this.releaseNudgeFn = resolve;
    });
  }
  releaseNudge(): void {
    this.nudgeReleased = true; // the live change is now visible to subsequent re-reads
    this.releaseNudgeFn();
  }
  async *subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta> {
    if (params.projection === "ApprovalQueue") {
      await this.nudgeGate; // hold the nudge until the test captures the baseline
      yield { projection: "ApprovalQueue", kind: "upsert", id: "appr_live_nudge" }; // row:None
      await new Promise<void>(() => {
        /* stay open — a live stream; the never-resolving promise is abandoned at test teardown. */
      });
      return;
    }
    yield* super.subscribe(params); // Session: the mock's benign delta + stay open
  }
  async get_projection<K extends ProjectionName>(
    name: K,
    scope?: ProjectionScope,
    page?: ProjectionPageParams,
  ): Promise<ProjectionPageByName[K]> {
    const result = await super.get_projection(name, scope, page);
    if (name === "ApprovalQueue") {
      this.approvalReads++;
      // The augmented approval appears ONLY after the nudge is released (the live change) — so ANY
      // pre-nudge read (the load, plus any recovery read) is the base set, regardless of count. This
      // ties the augmentation to the actual nudge, not a fragile read-index threshold.
      if (this.nudgeReleased) {
        const aq = result as ApprovalQueuePage;
        return { ...aq, rows: [...aq.rows, this.extra] } as ProjectionPageByName[K];
      }
    }
    return result;
  }
}

/** A gateway whose FIRST ApprovalQueue subscribe lag-closes (→ the supervisor degrades + recovers via
 *  re-get_projection), then stays open. Session stays healthy throughout. */
class RecoveringApprovalStreamGateway extends MockGatewayPort {
  private aqSubscribes = 0;
  async *subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta> {
    if (params.projection === "ApprovalQueue") {
      this.aqSubscribes += 1;
      if (this.aqSubscribes === 1) return; // first stream lag-closes → supervisor degrades + recovers
      await new Promise<void>(() => {
        /* the re-subscribe stays open (recovered) */
      });
      return;
    }
    yield* super.subscribe(params); // Session stays healthy (the degrade comes ONLY from ApprovalQueue)
  }
}

describe("Shell live ApprovalQueue subscribe (ui-059)", () => {
  it("approvalqueue_nudge_refetches_not_row_apply", async () => {
    // the lead-mandated pin: a row:None nudge drives a REFETCH whose rows replace the queue, NOT a
    // row-apply of the (absent) delta row. The extra pending approval can ONLY appear via the re-read.
    const extra = makeApprovalRow({
      approval_id: "approval_live_refetched",
      project_id: projectActivityFixture.rows[0]!.project_id, // a real project → counts in the switcher
      status: "awaiting_approval", // pending → +1 waitingOnYou
    });
    const gateway = new GatedApprovalRefetchGateway(extra);
    render(<Shell gateway={gateway} />);

    // load settles → the baseline waiting badge (the nudge is still gated → NO refetch yet).
    const badge = await screen.findByTestId("sidebar-waiting-badge");
    const baseline = Number(badge.textContent);
    const readsBeforeNudge = gateway.approvalReads;

    gateway.releaseNudge(); // the row:None nudge fires → coalesced refetch → augmented re-read

    await waitFor(() => {
      // the nudge caused a RE-READ (not a row-apply)…
      expect(gateway.approvalReads).toBeGreaterThan(readsBeforeNudge);
      // …and the re-read's extra pending approval is reflected in the live queue (badge +1).
      expect(
        Number(screen.getByTestId("sidebar-waiting-badge").textContent),
      ).toBe(baseline + 1);
    });
  });

  it("approvalqueue_subscribe_recovers_on_lag_close", async () => {
    // the ApprovalQueue stream reuses the 052 supervisor: a lag-close → reconnect → re-get_projection
    // (the ground-truth snapshot reset) → re-subscribe. The recovery refetch is observable as a 2nd
    // ApprovalQueue read (the 052 mechanism, now on the 2nd stream).
    const gateway = new RecoveringApprovalStreamGateway();
    const readSpy = vi.spyOn(gateway, "get_projection");
    render(<Shell gateway={gateway} />);

    await waitFor(
      () => {
        const aqReads = readSpy.mock.calls.filter(
          (c) => c[0] === "ApprovalQueue",
        ).length;
        expect(aqReads).toBeGreaterThanOrEqual(2); // load + the recovery refetch
      },
      { timeout: 3000 },
    );
  });
});

// ── ui-063 — whole-cockpit-live: the refetch-on-nudge spread to the REST ─────────────────────────────
// The remaining live-relevant served projections (ProjectActivity / PullRequest / UsageLedger) each get
// their own subscribe effect mirroring Session/ApprovalQueue. A daemon ProjectionDelta is a `row:None`
// id-NUDGE (deltas_for_event keys by project_id/pr_id/None) → the live cockpit must RE-READ
// get_projection, NOT apply the absent delta row (LESSON §29). Each gated gateway holds its projection's
// nudge until the test captures a baseline, then serves an AUGMENTED page on the post-nudge re-read, so
// the live change is observable only via the refetch. The OTHER projections stay healthy via super.

/** Gates the ProjectActivity `row:None` nudge; serves an AUGMENTED ProjectActivity page (one extra
 *  project) on the post-nudge re-read. A new project must re-key the switcher counts (ProjectActivity IS
 *  a deriveProjectSwitcherCounts input). Other projections stay healthy via super. */
class GatedProjectActivityRefetchGateway extends MockGatewayPort {
  projectReads = 0;
  private nudgeReleased = false;
  private releaseNudgeFn!: () => void;
  private readonly nudgeGate: Promise<void>;
  constructor(private readonly extra: ProjectActivityRow) {
    super();
    this.nudgeGate = new Promise<void>((resolve) => {
      this.releaseNudgeFn = resolve;
    });
  }
  releaseNudge(): void {
    this.nudgeReleased = true;
    this.releaseNudgeFn();
  }
  async *subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta> {
    if (params.projection === "ProjectActivity") {
      await this.nudgeGate; // hold the nudge until the test captures the baseline
      yield { projection: "ProjectActivity", kind: "upsert", id: "project_live_nudge" }; // row:None
      await new Promise<void>(() => {
        /* stay open — a live stream */
      });
      return;
    }
    yield* super.subscribe(params);
  }
  async get_projection<K extends ProjectionName>(
    name: K,
    scope?: ProjectionScope,
    page?: ProjectionPageParams,
  ): Promise<ProjectionPageByName[K]> {
    const result = await super.get_projection(name, scope, page);
    if (name === "ProjectActivity") {
      this.projectReads++;
      if (this.nudgeReleased) {
        const p = result as ProjectActivityPage;
        return { ...p, rows: [...p.rows, this.extra] } as ProjectionPageByName[K];
      }
    }
    return result;
  }
}

/** Gates the PullRequest `row:None` nudge; serves an AUGMENTED PullRequest page (one extra OPEN PR on the
 *  active project) on the post-nudge re-read → the TopBar openPRs count for the active project increments
 *  (PullRequest IS a deriveProjectSwitcherCounts input). Other projections stay healthy via super. */
class GatedPullRequestRefetchGateway extends MockGatewayPort {
  pullRequestReads = 0;
  private nudgeReleased = false;
  private releaseNudgeFn!: () => void;
  private readonly nudgeGate: Promise<void>;
  constructor(private readonly extra: PullRequestRow) {
    super();
    this.nudgeGate = new Promise<void>((resolve) => {
      this.releaseNudgeFn = resolve;
    });
  }
  releaseNudge(): void {
    this.nudgeReleased = true;
    this.releaseNudgeFn();
  }
  async *subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta> {
    if (params.projection === "PullRequest") {
      await this.nudgeGate;
      yield { projection: "PullRequest", kind: "upsert", id: "pr_live_nudge" }; // row:None
      await new Promise<void>(() => {
        /* stay open — a live stream */
      });
      return;
    }
    yield* super.subscribe(params);
  }
  async get_projection<K extends ProjectionName>(
    name: K,
    scope?: ProjectionScope,
    page?: ProjectionPageParams,
  ): Promise<ProjectionPageByName[K]> {
    const result = await super.get_projection(name, scope, page);
    if (name === "PullRequest") {
      this.pullRequestReads++;
      if (this.nudgeReleased) {
        const pr = result as PullRequestProjectionPage;
        return { ...pr, rows: [...pr.rows, this.extra] } as ProjectionPageByName[K];
      }
    }
    return result;
  }
}

/** Gates the UsageLedger `row:None` nudge; serves an AUGMENTED UsageLedger page (one extra usage row) on
 *  the post-nudge re-read. UsageLedger is NOT a deriveProjectSwitcherCounts input → a plain replace, NO
 *  recount; the refetch is observable as an incremented usage read. Other projections stay healthy via
 *  super. */
class GatedUsageRefetchGateway extends MockGatewayPort {
  usageReads = 0;
  private nudgeReleased = false;
  private releaseNudgeFn!: () => void;
  private readonly nudgeGate: Promise<void>;
  constructor(private readonly extra: UsageRow) {
    super();
    this.nudgeGate = new Promise<void>((resolve) => {
      this.releaseNudgeFn = resolve;
    });
  }
  releaseNudge(): void {
    this.nudgeReleased = true;
    this.releaseNudgeFn();
  }
  async *subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta> {
    if (params.projection === "UsageLedger") {
      await this.nudgeGate;
      // daemon-faithful: TelemetrySampled nudges are id-LESS (keyed None) — row AND id omitted.
      yield { projection: "UsageLedger", kind: "upsert" };
      await new Promise<void>(() => {
        /* stay open — a live stream */
      });
      return;
    }
    yield* super.subscribe(params);
  }
  async get_projection<K extends ProjectionName>(
    name: K,
    scope?: ProjectionScope,
    page?: ProjectionPageParams,
  ): Promise<ProjectionPageByName[K]> {
    const result = await super.get_projection(name, scope, page);
    if (name === "UsageLedger") {
      this.usageReads++;
      if (this.nudgeReleased) {
        const u = result as UsageProjectionPage;
        return { ...u, rows: [...u.rows, this.extra] } as ProjectionPageByName[K];
      }
    }
    return result;
  }
}

/** Gates the Review `row:None` nudge (ui-064); serves an AUGMENTED Review page (one extra review) on
 *  the post-nudge re-read. Reviews are NOT a counts input → plain replace; the refetch is observable as
 *  an incremented Review read. Other projections stay healthy via super. */
class GatedReviewRefetchGateway extends MockGatewayPort {
  reviewReads = 0;
  private nudgeReleased = false;
  private releaseNudgeFn!: () => void;
  private readonly nudgeGate: Promise<void>;
  constructor(private readonly extra: ReviewRow) {
    super();
    this.nudgeGate = new Promise<void>((resolve) => {
      this.releaseNudgeFn = resolve;
    });
  }
  releaseNudge(): void {
    this.nudgeReleased = true;
    this.releaseNudgeFn();
  }
  async *subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta> {
    if (params.projection === "Review") {
      await this.nudgeGate;
      yield { projection: "Review", kind: "upsert", id: "9999" }; // row:None
      await new Promise<void>(() => {
        /* stay open — a live stream */
      });
      return;
    }
    yield* super.subscribe(params);
  }
  async get_projection<K extends ProjectionName>(
    name: K,
    scope?: ProjectionScope,
    page?: ProjectionPageParams,
  ): Promise<ProjectionPageByName[K]> {
    const result = await super.get_projection(name, scope, page);
    if (name === "Review") {
      this.reviewReads++;
      if (this.nudgeReleased) {
        const rv = result as ReviewProjectionPage;
        return { ...rv, rows: [...rv.rows, this.extra] } as ProjectionPageByName[K];
      }
    }
    return result;
  }
}

describe("Shell whole-cockpit-live subscribe (ui-063 — refetch-on-nudge spread)", () => {
  it("projectactivity_subscribe_refetches_on_row_none_nudge", async () => {
    // spec(§6.1/§11): a daemon ProjectActivity delta is a `row:None` id-NUDGE → the live cockpit must
    // REFETCH get_projection("ProjectActivity"), NOT apply the absent row (LESSON §29). The new project
    // can ONLY appear via the re-read; ProjectActivity IS a switcher-counts input → counts re-key too.
    const extra: ProjectActivityRow = {
      project_id: "project_live_refetched",
      name: "Live new project",
    };
    const gateway = new GatedProjectActivityRefetchGateway(extra);
    render(<Shell gateway={gateway} />);

    // load settles → the nudge is still gated → the new project is NOT present yet (NO refetch).
    await screen.findByTestId("sidebar-waiting-badge");
    expect(screen.queryByText("Live new project")).toBeNull();
    const readsBeforeNudge = gateway.projectReads;

    gateway.releaseNudge(); // the row:None nudge fires → coalesced refetch → augmented re-read

    await waitFor(() => {
      // the nudge caused a RE-READ (not a row-apply)…
      expect(gateway.projectReads).toBeGreaterThan(readsBeforeNudge);
      // …and the re-read's extra project is now in the live tree.
      expect(screen.queryByText("Live new project")).not.toBeNull();
    });
  });

  it("pullrequest_subscribe_refetches_on_row_none_nudge", async () => {
    // spec(§6.1/§11): a `row:None` PullRequest nudge → REFETCH get_projection("PullRequest") (NOT
    // row-apply, LESSON §29); PullRequest IS a switcher-counts input → the active project's openPRs
    // count (TopBar) recomputes. The extra OPEN PR can ONLY count via the re-read.
    const activeProjectId = projectActivityFixture.rows[0]!.project_id; // the default active project
    const extra: PullRequestRow = {
      pr_id: "repo_live#999",
      project_id: activeProjectId,
      repo_id: "repo_live",
      pr_number: 999,
      title: "Live new PR",
      status: "open", // open → +1 openPRs for the active project
      head_branch: "feature/live",
      base_branch: "main",
      pr_checked_at: "2026-06-17T00:00:00Z",
      mergeable: true,
      checks_summary: "pending",
    };
    const gateway = new GatedPullRequestRefetchGateway(extra);
    render(<Shell gateway={gateway} />);

    await screen.findByTestId("sidebar-waiting-badge");
    // The TopBar renders the active project's openPRs count (title="open PRs").
    const baseline = Number(screen.getByTitle("open PRs").textContent);
    const readsBeforeNudge = gateway.pullRequestReads;

    gateway.releaseNudge(); // the row:None nudge fires → coalesced refetch → augmented re-read

    await waitFor(() => {
      // the nudge caused a RE-READ (not a row-apply)…
      expect(gateway.pullRequestReads).toBeGreaterThan(readsBeforeNudge);
      // …and the re-read's extra open PR is reflected in the live count (+1).
      expect(Number(screen.getByTitle("open PRs").textContent)).toBe(baseline + 1);
    });
  });

  it("review_subscribe_refetches_on_row_none_nudge", async () => {
    // ui-064 Layer 1 [§11.2]: Review joins the live-relevant served set (the one deferred from ui-063).
    // A `row:None` Review nudge (ReviewSynced) → REFETCH get_projection("Review") (NOT row-apply, §29).
    // Reviews are NOT a switcher-counts input → plain replace; the refetch is observable as a Review read.
    const extra: ReviewRow = {
      review_id: 9999,
      pr_number: 101,
      state: "approved",
      reviewer: "live",
    };
    const gateway = new GatedReviewRefetchGateway(extra);
    render(<Shell gateway={gateway} />);

    await screen.findByTestId("sidebar-waiting-badge");
    const readsBeforeNudge = gateway.reviewReads;

    gateway.releaseNudge(); // the row:None nudge fires → coalesced refetch → augmented re-read

    await waitFor(() => {
      // the nudge caused a RE-READ (not a row-apply) of the Review projection.
      expect(gateway.reviewReads).toBeGreaterThan(readsBeforeNudge);
    });
  });

  it("usageledger_subscribe_refetches_on_row_none_nudge", async () => {
    // spec(§6.1/§11): a `row:None` UsageLedger nudge → REFETCH get_projection("UsageLedger") (NOT
    // row-apply, LESSON §29). UsageLedger is NOT a switcher-counts input → a plain replace, NO recount;
    // the refetch is observable as an incremented usage read (the live telemetry surface stays current).
    const extra: UsageRow = {
      subject_id: "session_live_usage",
      harness: "claude",
      tokens: 5000,
      cost: 0.1,
      metric_quality: "exact",
      context_pct: 10,
    };
    const gateway = new GatedUsageRefetchGateway(extra);
    render(<Shell gateway={gateway} />);

    await screen.findByTestId("sidebar-waiting-badge");
    // The default Command Center renders a total-Tokens stat (sum of usage.tokens) — a cheap surface to
    // prove the refetched usage actually FLOWED into state (a refetch that forgot setData would still
    // bump reads). Captured as text so the assertion is robust to the exact fixture sum.
    const baselineTokens = screen.getByText("Tokens").parentElement!.textContent;
    const readsBeforeNudge = gateway.usageReads;

    gateway.releaseNudge(); // the row:None nudge fires → coalesced refetch → augmented re-read

    await waitFor(() => {
      // the nudge caused a RE-READ (not a row-apply) of the usage projection…
      expect(gateway.usageReads).toBeGreaterThan(readsBeforeNudge);
      // …and the re-read's extra usage row flowed into the live Tokens stat (plain replace, no recount).
      expect(screen.getByText("Tokens").parentElement!.textContent).not.toBe(baselineTokens);
    });
  });
});

// ── ui-064 Layer 2 — the live Review nudge flows into the RENDERED PR Review Workspace ───────────────
// Closes the L1 reviewer [medium]: at Layer 1 reviews had no rendered consumer (reads-increment was the
// only signal); now the PR Workspace renders the reviews-list for a selected PR, so a live Review nudge
// is observable as a rendered change (proving the refetch FLOWS into render, not just bumps a read).
describe("Shell PR Review Workspace live (ui-064 Layer 2)", () => {
  it("review_nudge_updates_rendered_pr_workspace_reviews", async () => {
    // [§11.2] select PR #101 in the Code view → its fixture reviews render → a Review `row:None` nudge
    // adds a review for #101 → it appears LIVE in the rendered reviews-list (refetch-on-nudge → render).
    const extra: ReviewRow = {
      review_id: 9999,
      pr_number: 101,
      state: "approved",
      reviewer: "liveReviewer",
      body: "ship it",
    };
    const gateway = new GatedReviewRefetchGateway(extra);
    const { container } = render(<Shell gateway={gateway} />);

    await screen.findByTestId("sidebar-waiting-badge");
    // navigate to the Code / Diff Review view → the Pull requests tab → select PR #101
    fireEvent.click(screen.getByRole("button", { name: /Code \/ Diff Review/i }));
    fireEvent.click(screen.getByRole("button", { name: /Pull requests/i }));
    const prCard = container.querySelector(
      '[data-item-id="PullRequest:repo_fixture_1#101"]',
    );
    expect(prCard).not.toBeNull(); // clear failure if the selecting-button locator ever changes
    fireEvent.click(prCard!);

    // selection worked → the PR Workspace shows #101's fixture reviews, but NOT the live one yet.
    await screen.findByText("alice");
    expect(screen.queryByText("liveReviewer")).toBeNull();

    gateway.releaseNudge(); // the row:None Review nudge → coalesced refetch → augmented re-read

    await waitFor(() => {
      // the live review for #101 flowed into the rendered reviews-list.
      expect(screen.queryByText("liveReviewer")).not.toBeNull();
    });
  });
});
