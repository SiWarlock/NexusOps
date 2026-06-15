//! nexusops-shared — the native Rust contract authority (Option A, ARCHITECTURE §5.0).
//!
//! Frozen in Phase 0.5 (OQ-DATA-SPIKE-5): the status state machines (§5.1), the
//! shared IDs + ULID-prefix format (§5.2), the actor enum (§7.1/R-2), and the
//! desktop-addendum objects (§5.3). `schemars` emits the versioned JSON Schema
//! consumed (generated) by the TS UI (Zod) and the Python Brain (Pydantic).

pub mod actions;
pub mod actor;
pub mod catalog;
pub mod event_envelope;
pub mod events;
pub mod gateway_ids;
pub mod harness;
pub mod ids;
pub mod ipc;
pub mod objects;
pub mod projections;
pub mod schema;
pub mod status;
pub mod time;

/// The frozen-contract version, stamped into the emitted JSON Schema and asserted
/// to agree across Rust / schema / Zod / Pydantic (the §5.0 propagation contract).
/// 0.9.0 added the IPC `GatewayPort` wire contract (§6.4: HelloFrame/HelloAck/
/// VersionSkewError/Capabilities/WireError + the IpcErrorCode + ProjectionName enums,
/// 1.5 L2). 0.10.0 (1.5 L3) adds the §6.1 RPC method envelopes (RpcRequest/RpcResponse/
/// GetProjectionParams/ProjectionScope) + the `protocol_error` code (the lead-ratified §6.4
/// gap resolution). 0.11.0 (1.5 L4) adds the frame-type multiplexing envelope (ServerFrame) +
/// subscribe streaming (ProjectionDelta/DeltaKind/SubscribeParams) — additive.
/// 0.12.0 (1.6a L3) adds the Device/LocalRunner registration event payloads
/// (DeviceRegistered/LocalRunnerRegistered + the DeviceId/LocalRunnerId newtypes,
/// §5.3/§16 bootstrap self-registration) — additive EventTypeRegistry rows.
/// 0.13.0 (1.6c L2) adds the §17 AuditIntegrityViolation event payload (Option C —
/// the loud, consumer-visible record emitted when startup replay quarantines a row).
/// 0.14.0 (1.7 L2) adds the §15 SensitiveOutputRedacted event payload — the
/// redaction "can't safely redact → divert the event + record this instead" net.
/// 0.15.0 (2.1a) freezes the §6.2 Gateway core data model (`shared/src/actions.rs`): the 10
/// ActionRequest/ActionPlan/Approval/ActionResult-family models + 9 new enums (RequesterType,
/// RiskLevel[bounded-int], ApprovalScope/Mode, PolicyDecisionStatus, ActionResultStatus,
/// ResourceType/EvidenceType/EvidenceConfidence) + the non-cross-product gateway IDs
/// (ApprovalId/ActionPlanId, off GatewayObjectKind) + the Timestamp newtype — additive, no
/// frozen type reshaped. Pure contract freeze; the Gateway pipeline/rows/events are 2.1b.
/// 0.16.0 (2.1b) adds the Gateway `ActionExecution*` EventTypeRegistry payloads (ActionRequested,
/// ActionApprovalRequested, ActionApproved, ActionDenied, ActionExpired, ActionStarted,
/// ActionSucceeded, ActionFailed) + the §6.1 `ActionAck` submit-result wire type — additive (the
/// chokepoint's authoritative event family + the ui/Brain intent-seam ack).
/// 0.17.0 (2.1c) adds the §6.1 `submit_action_plan` result wire types `PlanAck`/`PlanStepAck` (the
/// O-3 bundled-plan ack: the plan handle + per-step minted ids/status) — additive. The plan grouping
/// itself is daemon-internal (`action_plans` table + `plan_id` FK), NOT a new `shared/` type / event.
/// 0.18.0 (2.2) adds the §6.3 `ActionTypeCatalog` contract (`ActionTypeCatalogEntry` + the
/// `PreviewClass`/`ExecutorKind`/`IdempotencyFormula` enums, `shared/src/catalog.rs`) + the
/// `PolicyDecision` extension (`required_approvals`/`constraints`/`safer_alt`) — additive (the
/// catalog-driven policy engine's authoritative per-type risk + the richer decision payload).
/// 0.19.0 (2.4 L1) adds the §17 failure-mode contract: the `ActionPartiallySucceeded` event (the
/// side-effect-applied-but-terminal-event-unwritable record) + the structured `ActionError`
/// taxonomy now carried on `ActionFailed` (replacing the 2.1b free-string `error`) — additive.
/// 0.20.0 (3.1) freezes the §9.1 HarnessAdapter normalized return types (`shared/src/harness.rs`):
/// `TelemetrySample` + `MetricQuality` + `TranscriptRef` + `HarnessCapabilities` (10 PRD-HARN-5
/// fields) + the §7.1 `TelemetrySampled` telemetry-observation event — additive, no frozen type
/// reshaped. `NormalizedStatus` re-exports the frozen §5.1 `Session` (not a new type/$def). The
/// `HarnessAdapter` trait + `MutationIntercept` + the mutation-coverage matrix are DAEMON-INTERNAL
/// (not a `shared/` wire contract); `ResumeResult`/`ResumeMode`/`RecoveryState` survival froze at 4.1a (0.29.0).
/// 0.21.0 (3.4) freezes the §6.4 Terminal Channel wire contract (`shared/src/ipc.rs`): the 3 terminal
/// frames (`TerminalOutputFrame`/`TerminalInputFrame`/`TerminalControlFrame`) + the `TerminalControlKind`
/// (pause|resume) flow-control enum + the §7.1 `TerminalProcessExited` PTY-death observation event.
/// `ServerFrame` gains the `TerminalOutput` variant — the reserved Terminal slot filled with the
/// JSON-base64 MVP (raw PTY bytes base64 over the unchanged codec, LESSON §7); the binary fast-path is
/// a deferred 3.5 decision (additive — a future variant + bump, not a reshape). Additive, no frozen
/// type reshaped (§5.0). `terminal_id` = an opaque daemon runtime handle (`String` wire), NOT a 23rd
/// `IdKind`; the PTY host + backpressure pump are DAEMON-INTERNAL (`daemon/src/terminal/`).
/// 0.22.0 (3.2-part-2 / brief 043) extends the §6.3 ActionTypeCatalog for the Claude
/// `MutationIntercept`→Gateway interception (INV-SEC-1): a new `ExecutorKind::Adjudication` value (the
/// adjudication-only marker — the ActionRequest terminates at the verdict; no daemon executor runs the
/// tool) + the 4 `agent.*` catalog entries (`AGENT_MUTATION_ACTION_TYPES`, a SEPARATE machine-internal
/// const — the locked MVP-22 set is UNTOUCHED). Additive (a new enum value + a new action-type const;
/// no frozen type reshaped, §5.0). The `tool_name → agent.*` mapping + the params deny-rules + the
/// adjudication verdict are DAEMON-INTERNAL (`daemon/src/harness/claude/intercept.rs`).
/// 0.23.0 (043 L5 / A1) relaxes `ActionDenied.approval_id` `String`→`Option<String>` (`skip_serializing_if`):
/// a HUMAN-deny carries `Some(appr_…)`, an agent deny-rule **policy-deny** (denied at submit, before any
/// approval) carries `None` — the record-then-deny forensic event for a blocked dangerous agent attempt
/// (audit-integrity: never silently dropped). Additive-tolerant: a human-deny (`Some`) still serializes
/// the field identically; a policy-deny (`None`) OMITS it (`skip_serializing_if`), and a reader uses
/// `#[serde(default)]` to read it back as `None`. Un-consumed by ui today. §5.0.
/// 0.24.0 (P4.0b-1) freezes the **0.5b `ExecutionProfile`** runtime-state machine (the 10th §5.1
/// machine — HELD in 0.5 pending the cat-4 SDK-vs-PTY decision, now resolved = PTY-primary): 9 values
/// = the §5.1 8 + `credit_exhausted` (the SDK monthly credit-pool HARD-STOP, distinct from the soft
/// `rate_limited` interactive throttle; `disabled` is the only terminal). + adds
/// `SessionStarted.execution_profile_id` (`Option<ExecutionProfileId>`, the §15 #8 binding surface —
/// the profile recorded at session.create). Additive, no frozen type reshaped (§5.0).
/// 0.25.0 (P4.0b-1 L2) reclassifies the §6.3 catalog for the away-ruled risk-0 session-lifecycle:
/// `session.create` risk-2→risk-0 + NEW `session.kill` (risk-0) + NEW `session.profile_change`
/// (risk-2 — the §15 #8 no-silent-account-hop APPROVAL gate); `MVP_ACTION_TYPES` 22→24. The risk-0
/// relaxation is NARROW (an explicit daemon-policy auto-execute allowlist + a UI/IPC-only requester
/// guard). Catalog-data semantics; no frozen type reshaped (§5.0).
/// 0.26.0 (P4.0b-R1b) freezes the edges-R1 **Phase-5/7 wiring EventTypeRegistry payloads** (§7.1) — the
/// events edges' dormant executors emit (via `EmittedEvent`) + its projectors consume, in ONE batched
/// additive bump (edges regenerates once): P5.1 `ProjectRescanned` (one coarse event; `remote_url`
/// carries the §15 strip-userinfo-at-source contract); P5.2 `WorktreeCreated`/`BranchCreated` + the 4
/// empty-payload overlay transitions (`WorktreeMerged`/`WorktreePrunable`/`WorktreeDeleted`/
/// `WorktreeLocked`); P7.1 `PullRequestSynced` (reuses the §5.1 `PullRequest` enum) /
/// `IntegrationConnectionRegistered` (`keychain_ref` = a §15 pointer, never the secret) /
/// `GithubSyncFailed` + `LinearSyncFailed` (non-auth variant only; `reason` = a structural class-name) +
/// the closed `Provider` enum (`github`|`linear`). `shared/`-only — NO daemon emission (edges' P5/P7
/// executors emit later). Additive, no frozen type reshaped (§5.0).
/// 0.27.0 (P4.0b-2 — the live drive loop, user-ruled d.2 split tool-policy) adds **3 new `agent.*`
/// catalog types** to `AGENT_MUTATION_ACTION_TYPES`: `agent.todo_write` (risk-0 — the LONE benign-
/// internal auto-allow; the catalog IS the explicit enumerated allowlist, call-3 PIN) +
/// `agent.web_fetch`/`agent.web_search` (risk-2 require_approval — the network-EGRESS/exfil dimension).
/// Machine-internal (minted by the hook-receiver), so `MVP_ACTION_TYPES` stays 22 (the 043 precedent).
/// Additive — a new agent.* set + catalog entries; no frozen type reshaped (§5.0).
/// 0.28.0 (P4.0b-ui1 — the ui-6.3e unlock) adds: **3 new `git.*` MVP catalog types** (`git.stage_hunk`/
/// `git.unstage_hunk` risk-2; `git.discard_hunk` risk-3 + NON-standing-grantable, MVP 24→27) + the
/// `standing_grant_eligible` field on `ActionTypeCatalogEntry` (the §6.2 non-standing-grant floor,
/// unifying the risk-4 floor) + the §6.1 `get_diff` RPC types (`GetDiffParams`/`DiffResult`/`Hunk`/
/// `DiffLine`/`DiffLineKind`) + `IpcErrorCode::NotFound` (§6.4 9→10, the read-RPC not-found). Additive;
/// no frozen type reshaped (the catalog entry gains a field; existing entries default it true) (§5.0).
/// 0.29.0 (P4.1a) freezes the §8/§9.1 SURVIVAL contract (the deep-dive §7.2 B2-strict freeze):
/// `ResumeMode`(4: resumed|replayed|relaunched|reattached_live — the 4th = the user-ruled B2-strict §8
/// EXTENSION, the surviving in-flight turn reattached via the detachable-terminal broker) +
/// `RecoveryState`(3: recovering|recovered|recovery_failed, == the ui provisional, frozen verbatim) +
/// `ResumeResult{mode, replayed_event_count}` (replacing the daemon-internal `{resumed_live,…}`;
/// `replayed_event_count` non-zero only on the Replayed path). Additive, no frozen type reshaped (§5.0).
/// The `decide_resume` ladder is daemon-internal (4.1a commit 2); the broker subsystem is 4.1b.
/// 0.30.0 (P4.0b-ui2, the ②-mini, Fork B) freezes the FIRST projection-row — `ApprovalQueueRow`
/// (`shared/src/projections.rs`): the `proj_approval_queue` read model the §11.5 human-approval card
/// consumes, typed (the wire columns + `risk_level: RiskLevel` + `policy_decision: Option<PolicyDecision>`
/// — the §6.2 decision now persisted §15-redacted at approval-open + sibling-read into the row). Served
/// TYPED (no loose JSON on the approval path). Additive, no frozen type reshaped (§5.0).
/// 0.31.0 (P4.1b-1) adds the §8.1 daemon-restart `SessionRecovered` OBSERVATION event (`shared/src/events.rs`:
/// the §11.4 resumed-vs-replayed bit + the §17 "restart session" affordance; System-actor, write-actor,
/// NOT a Gateway Action [Q1=(a)]; §15 #8 profile preserved). Additive, no frozen type reshaped (§5.0).
/// 0.32.0 (P4.2) adds the §17 supervised-child-death `SessionFailed` OBSERVATION event (`shared/src/events.rs`:
/// a session's child died, daemon alive → proj_session status=Failed → the §11.4 restart affordance;
/// empty-payload, System-actor, write-actor, NOT a Gateway Action). Additive, no frozen type reshaped (§5.0).
/// 0.33.0 (P7.1 Wave-C, edges-029) adds the `integration.connect` §6.3 catalog action_type (risk-2,
/// registration-only) + `ExecutorKind::Integration` (`shared/src/catalog.rs`). EDGES-BRANCH-LOCAL: the
/// daemon (catalog/CONTRACT owner) RATIFIES the action_type + assigns the final version at the
/// edges→main merge (like the MIGRATION numbers). Additive, no frozen type reshaped (§5.0).
pub const CONTRACT_VERSION: &str = "0.33.0";
