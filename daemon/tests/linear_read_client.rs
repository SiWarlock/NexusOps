//! P7.1 — the Linear read client: deterministic core + seam (edges-014). Opens the Linear read
//! vertical's client layer (the analog of edges-009's GitHub PR read client). All in-lane; the real
//! reqwest/GraphQL fetch (`LinearGraphqlReadClient`) + the gated `tasks`(external_task) projector +
//! the `linear.*` executors stay deferred (edges-015 / R1 daemon seam).
//!
//! The DETERMINISTIC core — `extract_issue(&str) -> Option<LinearIssue>` — is fixture-driven
//! test-first: a recorded **public** Linear GraphQL issue response (`{ data: { issue: {...} } }`)
//! deserializes through PRIVATE wire structs, then the issue node maps into `LinearIssue`, deriving
//! the §5.1 `Task` status via edges-013 (`derive_task_status_from_linear ∘ parse_linear_state_type`
//! — the single status authority). The live HTTP round-trip is the non-deterministic edge, covered by
//! `FakeLinearReadClient` (CLAUDE.md: real Linear calls → recorded fixtures/fakes; never a real key).
//!
//! Linear GraphQL reality (confirmed via Context7 against linear.app/developers): `POST
//! https://api.linear.app/graphql`; `issue(id){ id identifier title url state{ type name }
//! assignee{ id name } }` → `{ "data": { "issue": {…} } }`; `state.type` ∈ the closed 6-value set
//! (triage/backlog/unstarted/started/completed/canceled — edges-013); `assignee` nullable.

use nexusops_shared::status::Task;
use nexusops_shared::time::Timestamp;
use nexusopsd::integrations::classifier::IntegrationOutcomeClass;
use nexusopsd::integrations::linear::{
    extract_issue, FakeLinearReadClient, LinearReadClient, LinearReadError, LinearTeam,
};

// ---- recorded public-shape Linear GraphQL responses (inline const — hermetic, no fixture-file IO) -

// state.type="started" + a present assignee. The custom state.name ("In Review") is deliberately a
// DIFFERENT token from the type ("started") — so the same fixture pins §B-#5: status derives from the
// closed `state.type`, NEVER the team's free-form `state.name`. (Identity-map fixture too.)
const ISSUE_STARTED: &str = r#"{"data":{"issue":{"id":"a1b2c3d4-0000-0000-0000-000000000001","identifier":"BLA-123","title":"Update documentation for API","url":"https://linear.app/acme/issue/BLA-123","state":{"type":"started","name":"In Review"},"assignee":{"id":"u_001","name":"Ada Lovelace"}}}}"#;
// state.type="completed".
const ISSUE_COMPLETED: &str = r#"{"data":{"issue":{"id":"a1b2c3d4-0000-0000-0000-000000000002","identifier":"BLA-124","title":"Ship the release","url":"https://linear.app/acme/issue/BLA-124","state":{"type":"completed","name":"Done"},"assignee":{"id":"u_001","name":"Ada Lovelace"}}}}"#;
// assignee: null (unassigned).
const ISSUE_NO_ASSIGNEE: &str = r#"{"data":{"issue":{"id":"a1b2c3d4-0000-0000-0000-000000000003","identifier":"BLA-125","title":"Triage inbound","url":"https://linear.app/acme/issue/BLA-125","state":{"type":"unstarted","name":"Todo"},"assignee":null}}}"#;
// state.type unrecognized → the edges-013 conservative floor (Backlog → Queued).
const ISSUE_WEIRD_STATE: &str = r#"{"data":{"issue":{"id":"a1b2c3d4-0000-0000-0000-000000000004","identifier":"BLA-126","title":"Mystery state","url":"https://linear.app/acme/issue/BLA-126","state":{"type":"intergalactic","name":"Custom State"},"assignee":null}}}"#;
// data.issue null (issue not found / empty) → the None arm of the Option return.
const ISSUE_NULL: &str = r#"{"data":{"issue":null}}"#;
// edges-018 richer fields — a full issue with description, priority 2, a team {id,name,key}, and
// ISO-8601 created/updated timestamps, alongside the existing selection.
const ISSUE_RICH: &str = r#"{"data":{"issue":{"id":"a1b2c3d4-0000-0000-0000-000000000010","identifier":"ENG-200","title":"Rich issue","url":"https://linear.app/acme/issue/ENG-200","state":{"type":"started","name":"In Progress"},"assignee":{"id":"u_001","name":"Ada Lovelace"},"description":"A detailed issue body.","priority":2,"team":{"id":"team_eng","name":"Engineering","key":"ENG"},"createdAt":"2026-06-01T10:00:00.000Z","updatedAt":"2026-06-02T11:30:00.000Z"}}}"#;
// priority 9 (outside the 0–4 range) → priority maps to None; the rest of the issue still maps.
const ISSUE_BAD_PRIORITY: &str = r#"{"data":{"issue":{"id":"a1b2c3d4-0000-0000-0000-000000000011","identifier":"ENG-201","title":"Bad priority","url":"https://linear.app/acme/issue/ENG-201","state":{"type":"backlog","name":"Backlog"},"assignee":null,"priority":9}}}"#;
// createdAt is not ISO-8601 → created_at None; updatedAt is valid → Some (per-field parse degradation).
const ISSUE_BAD_TIMESTAMP: &str = r#"{"data":{"issue":{"id":"a1b2c3d4-0000-0000-0000-000000000012","identifier":"ENG-202","title":"Bad timestamp","url":"https://linear.app/acme/issue/ENG-202","state":{"type":"started","name":"Doing"},"assignee":null,"createdAt":"yesterday","updatedAt":"2026-06-02T11:30:00Z"}}}"#;

// ---- extraction fixtures → derived §5.1 status (the deterministic core) -------------------------

#[test]
fn extract_started_issue_is_in_progress() {
    // spec(§9/§5.1): the response → issue → derive chain — a "started" issue extracts to InProgress
    // via edges-013 (derive_task_status_from_linear ∘ parse_linear_state_type), the single authority.
    let issue = extract_issue(ISSUE_STARTED).expect("a well-formed issue node");
    assert_eq!(issue.status, Task::InProgress);
}

#[test]
fn extract_completed_issue_is_done() {
    // spec(§5.1): a "completed" issue → Task::Done (the work-complete mapping; Done is non-terminal).
    let issue = extract_issue(ISSUE_COMPLETED).expect("a well-formed issue node");
    assert_eq!(issue.status, Task::Done);
}

#[test]
fn extract_issue_missing_assignee_is_none() {
    // spec(§8): total over the optional field — assignee:null → assignee: None (an external_task row
    // with no assignee), never a panic or a fabricated name.
    let issue = extract_issue(ISSUE_NO_ASSIGNEE).expect("a well-formed issue node");
    assert_eq!(issue.assignee, None);
}

#[test]
fn extract_issue_preserves_identity_fields() {
    // spec(§8/§5.1): the external_task row fields — id/identifier/title/url/state_name map through
    // verbatim, and a present assignee surfaces as its name. The SAME fixture pins the §B-#5
    // independence: state.name="In Review" (a custom label) is preserved verbatim as state_name, while
    // status derives from state.type="started" → Task::InProgress — proving status is NOT read off the
    // free-form name (which, if mis-derived, would suggest a review-flavored status).
    let issue = extract_issue(ISSUE_STARTED).expect("a well-formed issue node");
    assert_eq!(issue.id, "a1b2c3d4-0000-0000-0000-000000000001");
    assert_eq!(issue.identifier, "BLA-123");
    assert_eq!(issue.title, "Update documentation for API");
    assert_eq!(issue.url, "https://linear.app/acme/issue/BLA-123");
    assert_eq!(issue.state_name, "In Review"); // verbatim custom name
    assert_eq!(issue.status, Task::InProgress); // derived from type, NOT name (§B-#5)
    assert_eq!(issue.assignee.as_deref(), Some("Ada Lovelace"));
}

#[test]
fn extract_unknown_state_type_floors() {
    // spec(§5.1): an unrecognized state.type survives the extraction at the edges-013 conservative
    // floor (Backlog → Queued) — a parse-miss API anomaly parks, never fabricating human-attention.
    let issue = extract_issue(ISSUE_WEIRD_STATE).expect("a well-formed issue node");
    assert_eq!(issue.status, Task::Queued);
}

#[test]
fn extract_absent_issue_is_none() {
    // spec(§9): the Option None arm — every "no well-formed issue node" shape yields None (the
    // &str -> Option total contract; pure, never a panic): data.issue null (not found), `data` itself
    // null, a GraphQL-errors body with no `data` key (edges-015 classifies THAT separately via
    // `classify` — the §17 forward-constraint extract_issue must not swallow fidelity from), and a
    // non-JSON body.
    assert_eq!(extract_issue(ISSUE_NULL), None); // {"data":{"issue":null}}
    assert_eq!(extract_issue(r#"{"data":null}"#), None); // data itself null
    assert_eq!(
        extract_issue(r#"{"errors":[{"message":"Authentication required"}]}"#),
        None // GraphQL-errors body (no data key) — edges-015 owns the errors[] classification
    );
    assert_eq!(extract_issue("not json"), None);
}

// ---- edges-018 richer secondary signals (description/priority/team/timestamps) -------------------

#[test]
fn extract_issue_maps_richer_fields() {
    // spec(§9): the user-directed completeness mapping — description/priority/team/created+updated all
    // populate from a full fixture; status stays state.type-derived (edges-013, the single authority).
    let issue = extract_issue(ISSUE_RICH).expect("a well-formed issue node");
    assert_eq!(issue.status, Task::InProgress); // edges-013, unchanged
    assert_eq!(issue.description.as_deref(), Some("A detailed issue body."));
    assert_eq!(issue.priority, Some(2));
    assert_eq!(
        issue.team,
        Some(LinearTeam {
            id: "team_eng".to_owned(),
            key: "ENG".to_owned(),
            name: "Engineering".to_owned(),
        })
    );
    assert_eq!(
        issue.created_at,
        Some(Timestamp::parse("2026-06-01T10:00:00.000Z").expect("a valid ts"))
    );
    assert_eq!(
        issue.updated_at,
        Some(Timestamp::parse("2026-06-02T11:30:00.000Z").expect("a valid ts"))
    );
}

#[test]
fn extract_issue_richer_fields_absent_are_none() {
    // spec(§8 backward-compat): an OLD minimal fixture (no new fields) STILL deserializes — the tolerant
    // wire (Option + serde-default) reads the new fields as None, and the existing fields
    // (status/state_name/assignee) are UNCHANGED.
    let issue = extract_issue(ISSUE_STARTED).expect("a well-formed issue node");
    assert_eq!(issue.description, None);
    assert_eq!(issue.priority, None);
    assert_eq!(issue.team, None);
    assert_eq!(issue.created_at, None);
    assert_eq!(issue.updated_at, None);
    // existing fields unchanged:
    assert_eq!(issue.status, Task::InProgress);
    assert_eq!(issue.state_name, "In Review");
    assert_eq!(issue.assignee.as_deref(), Some("Ada Lovelace"));
}

#[test]
fn extract_issue_priority_out_of_range_is_none() {
    // spec(§8 total mapping): priority 9 (outside 0–4) → None — total/no-panic over an unexpected wire
    // value; the rest of the issue still maps (status via edges-013).
    let issue = extract_issue(ISSUE_BAD_PRIORITY).expect("a well-formed issue node");
    assert_eq!(issue.priority, None);
    assert_eq!(issue.status, Task::Queued); // backlog → Queued, unaffected
}

#[test]
fn extract_issue_malformed_timestamp_is_none() {
    // spec(§8): a non-ISO-8601 createdAt → created_at None (Timestamp::parse failure degrades to None,
    // mirrors CommitInfo.timestamp; never panics); a valid updatedAt alongside still maps to Some.
    let issue = extract_issue(ISSUE_BAD_TIMESTAMP).expect("a well-formed issue node");
    assert_eq!(issue.created_at, None);
    // the valid updatedAt (millisecond-free `Z` form) alongside still parses to its exact instant:
    assert_eq!(
        issue.updated_at,
        Some(Timestamp::parse("2026-06-02T11:30:00Z").expect("a valid ts"))
    );
}

// ---- the trait seam (the gated tasks projector / §7.3 Task Inbox consume this) -------------------

#[tokio::test]
async fn fake_client_returns_canned_issue() {
    // spec(§8): FakeLinearReadClient is the seam the gated tasks(external_task) projector + §7.3 Task
    // Inbox consume in tests — it returns the canned Ok(issue) (the live HTTP fetch is edges-015).
    let issue = extract_issue(ISSUE_STARTED).expect("a well-formed issue node");
    let fake = FakeLinearReadClient::new(Ok(issue.clone()));
    let got = fake.fetch_issue("BLA-123").await;
    assert_eq!(got, Ok(issue));
}

#[tokio::test]
async fn fake_client_returns_canned_error() {
    // spec(§17): the error carries IntegrationOutcomeClass (NOT the collapsed DeliveryOutcome) so the
    // gated SyncFailed/auth_expired path can branch on AuthFailed vs a payload ClientError.
    let err = LinearReadError {
        class: IntegrationOutcomeClass::AuthFailed,
        message: "401 Unauthorized".to_owned(),
    };
    let fake = FakeLinearReadClient::new(Err(err.clone()));
    let got = fake.fetch_issue("BLA-123").await;
    assert_eq!(got, Err(err));
    // the §17 forward-constraint: the carried class is reachable + distinct (AuthFailed, not a
    // collapsed terminal) — the field the gated auth_expired path branches on.
    assert_eq!(got.unwrap_err().class, IntegrationOutcomeClass::AuthFailed);
}
