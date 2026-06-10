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

export const ActionRequest = shape.ActionRequest;
export const ActorType = shape.ActorType;
export const AgentTeam = shape.AgentTeam;
export const Approval = shape.Approval;
// 0.12.0 additions (daemon 1.5/1.6 IPC + projection-name freeze).
export const DeltaKind = shape.DeltaKind;
export const DesktopObjectKind = shape.DesktopObjectKind;
export const IdKind = shape.IdKind;
export const IpcErrorCode = shape.IpcErrorCode;
// Exported as ...Enum: the provisional registry KEY TYPE `ProjectionName`
// (keyof ProjectionPageByName, re-exported below) keeps the bare name until the
// page-shape reconcile retires it; the validators record key stays schema-exact.
export const ProjectionNameEnum = shape.ProjectionName;
export const ProjectBrain = shape.ProjectBrain;
export const PullRequest = shape.PullRequest;
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

/** Name → generated enum validator, for drift-checking against the frozen schema. */
export const validators: Record<string, EnumValidator> = {
  ActionRequest,
  ActorType,
  AgentTeam,
  Approval,
  DeltaKind,
  DesktopObjectKind,
  IdKind,
  IpcErrorCode,
  ProjectionName: ProjectionNameEnum,
  ProjectBrain,
  PullRequest,
  RedactionStatus,
  Sensitivity,
  Session,
  SourceType,
  Task,
  Visibility,
  WorkflowInstance,
  WorktreeGit,
  WorktreeOverlay,
};

export { CONTRACT_VERSION } from "./generated";
export * from "./provisional";
