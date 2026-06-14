// Shared projection→item mappers (§11.2/§11.3). The single source for the chrome
// + view item shape: an entity reduced to its locator id + display label +
// (machine,status) for the descriptor table. The Sidebar, the Command Center,
// and the graph's session/PR nodes all route through these — no view re-maps a
// projection row inline. Attention is NEVER derived here; callers pass the
// (machine,status) pair to describeStatus (the single descriptor source).
import type {
  SessionRow,
  PullRequestRow,
  ApprovalQueueRow,
} from "../contracts/index";

/**
 * An entity reduced to its locator id, display label, and (machine,status).
 *
 * `machine` is one of the §5.1 status-machine names the descriptor table is
 * keyed on — here `"Session"` | `"PullRequest"` | `"Approval"`. `status` is the
 * source row's frozen-enum value, intentionally WIDENED to `string` for this
 * shared shape: `describeStatus(machine, status)` is itself string-keyed (with
 * an unknown→visible fallback), so the render path never needs the narrow type.
 * Narrowing `machine`/`status` to the generated enum unions is a follow-up tied
 * to the provisional→generated object-schema reconcile (Carry-forward).
 */
export interface ProjectionItem {
  id: string;
  label: string;
  machine: string;
  status: string;
}

export function toSessionItems(rows: SessionRow[]): ProjectionItem[] {
  return rows.map((s) => ({
    id: s.session_id,
    label: s.title ?? s.session_id,
    machine: "Session",
    status: s.status,
  }));
}

export function toPrItems(rows: PullRequestRow[]): ProjectionItem[] {
  return rows.map((pr) => ({
    id: pr.pr_number,
    label: pr.title ?? `PR #${pr.pr_number}`,
    machine: "PullRequest",
    status: pr.status,
  }));
}

export function toApprovalItems(rows: ApprovalQueueRow[]): ProjectionItem[] {
  return rows.map((a) => ({
    id: a.approval_id,
    // the frozen ApprovalQueueRow has no `title` — `preview_summary` is the human label.
    label: a.preview_summary ?? a.approval_id,
    machine: "Approval",
    status: a.status,
  }));
}
