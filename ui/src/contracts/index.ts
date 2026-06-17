// The UI contract layer — the single import surface for frozen-contract enums
// and provisional object shapes.
//
// The individual enum validators below are DERIVED from the generated bundle
// (src/contracts/generated.ts, itself regenerated from the frozen shared schema
// via `pnpm gen:contracts`). They are never hand-declared — the generated layer
// is the only source, and the drift test pins it to the frozen schema.
// ARCHITECTURE.md §5.0 (generated, drift-caught consumer); §11.3 (status keys
// == §5.1 enum strings, verbatim snake_case).
import bundle from "./generated";

const shape = bundle.shape;

// 0.19.0 rename (Phase-2 Gateway freeze): the schema renamed the two R-5 status
// value-sets to their `$def` names (was bare `ActionRequest`/`Approval`). The
// UI's status-machine identifiers (`"ActionRequest"`/`"Approval"`) are a separate
// UI-render-policy naming (descriptor table keys / data-machine) and DON'T follow.
export const ActionRequestStatus = shape.ActionRequestStatus;
export const ActorType = shape.ActorType;
export const AgentTeam = shape.AgentTeam;
export const ApprovalStatus = shape.ApprovalStatus;
// 0.12.0 additions (daemon 1.5/1.6 IPC + projection-name freeze).
export const DeltaKind = shape.DeltaKind;
export const DesktopObjectKind = shape.DesktopObjectKind;
// 0.28.0 (§6.3e get_diff): DiffLineKind is consumed by the DiffLine provisional
// shape. ExecutionProfile/Provider are also generated at 0.28.0 but exposed-ahead
// (no consumer yet) → not re-exported here until a typed consumer lands.
export const DiffLineKind = shape.DiffLineKind;
export const IdKind = shape.IdKind;
export const IpcErrorCode = shape.IpcErrorCode;
// Exported as ...Enum: the provisional registry KEY TYPE `ProjectionName`
// (keyof ProjectionPageByName, re-exported below) keeps the bare name until the
// page-shape reconcile retires it; the validators record key stays schema-exact.
export const ProjectionNameEnum = shape.ProjectionName;
export const ProjectBrain = shape.ProjectBrain;
export const PullRequest = shape.PullRequest;
// 0.38.0 (D5b-1 review vertical): ReviewState is consumed by the ReviewRow provisional
// shape (ui-061) — the exposed-ahead→re-export-on-consume pattern (DiffLineKind@0.28).
export const ReviewState = shape.ReviewState;
// 0.8.0 additions (daemon Phase 1 event-store/redaction contract surface).
export const RedactionStatus = shape.RedactionStatus;
export const Sensitivity = shape.Sensitivity;
export const Session = shape.Session;
export const SourceType = shape.SourceType;
export const Task = shape.Task;
export const Visibility = shape.Visibility;
export const WorkflowInstance = shape.WorkflowInstance;
export const WorktreeGit = shape.WorktreeGit;
export const WorktreeOverlay = shape.WorktreeOverlay;

type EnumValidator = (typeof shape)[keyof typeof shape];

/**
 * Name → generated enum validator, for drift-checking against the frozen schema.
 * DERIVED from the generated bundle (every value-set, self-maintaining across
 * contract bumps) — never hand-listed. A hand-list is exactly what drifted at
 * 0.12.0 (20 listed vs 33 frozen); deriving keeps the §5.0 "keys equal" drift
 * check self-maintaining so a future bump is a pure `pnpm gen:contracts`.
 */
export const validators: Record<string, EnumValidator> = shape;

export { CONTRACT_VERSION } from "./generated";
export * from "./provisional";
export * from "./intent-contracts";
