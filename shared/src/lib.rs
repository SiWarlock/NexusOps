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
/// 0.34.0 (P7.2) freezes the 2nd projection-row — `PullRequestRow` (`shared/src/projections.rs`): the
/// `proj_pull_request` GitHub-authoritative read cache the ui PR Review Workspace (§11.2/§7.2) consumes,
/// typed (the BASIC columns the edges-P7.1 projector folds; `status: PullRequest`). Served TYPED (no
/// loose JSON; `read_pull_request_typed`, fail-closed). `mergeable`/`checks_summary` are a later SPREAD
/// (no projection column yet). Additive, no frozen type reshaped (§5.0).
/// 0.35.0 (D2/P4.4) freezes the 3rd projection-row — `SessionRow` (`shared/src/projections.rs`): the
/// `proj_session` read model the ui per-session recovery indicator + `RecoveryState` banner (§11.4)
/// consume, typed (`status: Session` + the §8.1/§11.4 recovery fields `resume_mode`/
/// `replayed_event_count`/`recovered_at` folded from the now-CONSUMED `SessionRecovered`). Served TYPED
/// (`read_session_typed`, fail-closed). The not-yet-consumed `proj_session` columns are a later SPREAD.
/// Additive, no frozen type reshaped (§5.0).
/// 0.36.0 (D5a/P4.6) enriches the `PullRequestRow` projection-row (`shared/src/projections.rs`) with
/// `mergeable: Option<bool>` + `checks_summary: Option<String>` — the P7.2 "basic-now + SPREAD" consumed:
/// folded from `PullRequestSynced.mergeable?`/`checks_summary?` into the 2 new `proj_pull_request` columns
/// (MIGRATION_13, ALTER-only) + served TYPED (`mergeable` is the first bool projection column — INTEGER
/// 0/1 in SQLite, coerced to a JSON bool in the daemon read layer; the contract stays a pure `Option<bool>`).
/// Additive, no frozen type reshaped (§5.0).
/// 0.37.0 (D5b-1/P4.6) freezes the structured-review vertical (`shared/src/{events,status,projections,ipc}.rs`):
/// the `ReviewSynced` event + the `ReviewState` value enum (snake_case, reject-unknown — NOT a status_machine;
/// a review is a fixed verdict, no lifecycle) + the 4th frozen projection-row `ReviewRow` (the `proj_review`
/// read model the §11.2 PR Review Workspace consumes, served TYPED) + `ProjectionName::Review` (the §6.1
/// closed-set add — a subscribe-able projection). Fed by synthetic events; the live GitHub producer is D5b-2.
/// Additive, no frozen type reshaped (§5.0).
/// 0.38.0 (D5b-2/P4.6) adds the `github.sync_reviews` §6.3 catalog action_type (`shared/src/catalog.rs`;
/// `MVP_ACTION_TYPES` 28→29) — the live review producer (its `GithubExecutor` arm fetches a PR's reviews +
/// emits one `ReviewSynced` each). **risk-1, standing_grant_eligible=true** — the PRECEDENT for github
/// network READS: not risk-0 auto-execute (an external API read), below the risk-2 github writes (no
/// mutation/credential). Reuses `ExecutorKind::Github` (no new enum → the 3-way verify count is unchanged).
/// Additive, no frozen type reshaped (§5.0).
/// 0.39.0 (D6/P4.7) adds the PR-card diff-stats — `additions`/`deletions`/`changed_files`/`commits`
/// (`Option<u64>`) on `PullRequestSynced` (§7.1) + `PullRequestRow` (§7.2), folded onto `proj_pull_request`
/// (MIGRATION_15) + served typed; captured from the octocrab GET PR in `extract_pr_signals` + threaded
/// into the `create_pr` emit (the §11.2 PR card render data). The D5a LOCKSTEP recipe (LESSON §53);
/// additive, no frozen type reshaped (§5.0).
/// 0.40.0 (D7/P4.7) adds the `get_pr_diff` §6.1 read RPC — a NEW `GetPrDiffParams` wire type
/// (`{repo_id, pr_number, file?}`) for the §11.2 Review-tab remote-PR code-diff (head-vs-base);
/// `DiffResult`/`Hunk`/`DiffLine` are REUSED (no new result shape). The first network read in the IPC
/// read layer (an `Arc<dyn GithubReadClient>` threaded into the dispatch). Additive (§5.0).
/// 0.41.0 (D9/P4.7) adds the cat-1 `github.merge_pr` mutation surface: the §6.3 catalog entry (risk-3,
/// `ExecutorKind::Github`, FromInputs, requires_resource_refs, **NON-standing-grantable** — F1; MVP set
/// 29→30) + the §7.1 `PullRequestMerged` OBSERVATION event (`{pr_number, merge_commit_sha?, merged_at}`,
/// emitted by the gateway on a successful merge; the `PullRequestProjector` folds it → terminal `Merged`).
/// The F2 UI/IPC-only requester gate is a daemon policy concern (no `shared/` surface). Additive, no
/// frozen type reshaped (§5.0).
/// 0.42.0 (D10/P4.7) adds the cat-1 `github.submit_review` mutation surface: the §6.3 catalog entry (risk-3,
/// `ExecutorKind::Github`, FromInputs, requires_resource_refs, **NON-standing-grantable** — F1; MVP set
/// 30→31) + the §7.1 `ReviewSubmitted` OBSERVATION event (`{review_id, pr_number, reviewer, state, body?,
/// submitted_at?, commit_id?}`, the WRITE counterpart to `ReviewSynced`, reusing the frozen `ReviewState`;
/// emitted by the gateway on a successful review-submit; the `ReviewProjector` folds it → `proj_review`).
/// The F2 UI/IPC-only requester gate extends the D9 daemon-policy const (no `shared/` surface). Additive,
/// no frozen type reshaped (§5.0).
/// 0.43.0 (P5.3a) adds the §15 #8 `ExecutionProfileRegistered` System-actor event — the FIRST
/// DATA_MODEL-§2.8 canonical OBJECT registry (`execution_profiles`, Option B durable row = source of
/// truth, the event is the audit trail). `keychain_ref` is a §15 #4 POINTER (no token; secret WRITE =
/// 5.3b). Additive EventTypeRegistry row, no frozen type reshaped (§5.0).
/// 0.44.0 (P4.7) adds `head_sha: Option<String>` to `PullRequestSynced` + `PullRequestRow` (the anti-race
/// SHA-pin SOURCE the UI reads — captured only from `pr.head.sha`, never a proposer). Additive LOCKSTEP
/// row enrichment (the D6 diff-stats precedent), no frozen type reshaped (§5.0).
/// 0.45.0 (P4.7/083) adds the auth-bootstrap surface: (a) the live-writes toggle vertical — the §6.3
/// catalog entry `integration.set_live_writes` (risk-2, `ExecutorKind::Integration`, FromInputs,
/// requires_resource_refs=false, **NON-standing-grantable** — the §6.2 credential/live-enablement floor;
/// MVP set 31→32) + the §7.1 `IntegrationLiveWritesSet` event (`{connection_id, enabled}` — NO secret; the
/// `IntegrationConnection` projector folds it → `proj_integration_connection.live_writes_enabled`, default
/// OFF); and (b) the §6.1 `connect_via_gh` "Connect via gh" trigger wire types — `ConnectViaGhParams`
/// (`{provider, account}`, NO token — the daemon sources it), `ConnectViaGhResult` (`{status, keychain_ref?}`
/// — the keychain POINTER only), `ConnectViaGhStatus` (`connected|gh_unavailable`). The F2 UI/IPC-only
/// requester gate is a daemon policy concern (no `shared/` surface). Additive, no frozen type reshaped (§5.0).
/// 0.46.0 (P5.3b/085) adds the execution-profile SECRET vertical: (a) the §6.3 catalog entry
/// `profile.set_keychain_ref` (risk-2, the NEW `ExecutorKind::Profile`, NaturalResourceRef,
/// requires_resource_refs=true, **NON-standing-grantable** — the §6.2 credential floor; MVP set 32→33) — the
/// typed Gateway action recording the §15 #4 keychain POINTER onto the canonical `execution_profiles` row;
/// (b) the §7.1 `ProfileSecretSet` (`{execution_profile_id}` — NO secret; the keychain pointer is daemon-derived) + `SessionProfileChanged`
/// (`{session_id, execution_profile_id}` — the §15 #8 no-account-hop rebind) events; (c) the §6.1
/// `profile.set_secret` inbound-secret IPC trigger wire types — `SetProfileSecretParams` (`{execution_profile_id,
/// secret}` — the FIRST inbound `secret`, getpeereid-authed, Zeroizing daemon-side), `SetProfileSecretResult`
/// (`{keychain_ref}` — the POINTER only, no echo). Additive, no frozen type reshaped (§5.0).
/// 0.47.0 (P4.7/092) adds `ProjectRescanned.name: Option<String>` — the human-readable project name (the
/// basename of the scan path) surfaced into `proj_project_activity.name` (MIGRATION_19) for the cockpit
/// switcher label. Additive-optional (pre-092 events replay as `None`), no frozen type reshaped (§5.0).
/// 0.48.0 (W1-prof/093) adds the `get_execution_profiles` read RPC — the secret-free `ProfileRow`
/// (`{execution_profile_id, provider, harness, model?, account_alias?, status, is_default, has_credential}`
/// — §15 #4: NO keychain_ref/secret; credential state as the derived `has_credential` bool) +
/// `GetExecutionProfilesResult` (`{profiles}`) IPC wire types — the §2.8 `execution_profiles` registry read
/// surface the cockpit profile picker consumes. Additive, no frozen type reshaped (§5.0).
pub const CONTRACT_VERSION: &str = "0.48.0";
