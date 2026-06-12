//! Workflow / file-signal detection — pure filesystem presence checks.
//!
//! `detect_workflow` reports which scaffolding / plan / Brain signals a project directory carries,
//! feeding the `project.rescan` executor (wired in edges-002). All checks are presence-only and
//! infallible (a bare directory → all signals absent). Detection is **advisory** (§9): a basic
//! project with no pack still works.

use std::path::{Path, PathBuf};

/// Scaffolding / plan / Brain signals detected at a project root (§7.2 SoT markers).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkflowDetection {
    /// A workflow pack — `.scaffolding/manifest.json` present (§7.2 WorkflowInstance SoT).
    pub workflow_pack: bool,
    /// cc-crew scaffolding — a `.claude/` directory present.
    pub cc_crew: bool,
    /// The plan file + its path, when present (§7.2 PlanTask SoT).
    pub plan_file: Option<PathBuf>,
    /// A Project Brain association — presence-only (full Brain status = Phase 8 brainclient).
    pub brain: bool,
}

/// Candidate plan-file names, in precedence order: `MVP_TASKS.md` is the §7.2 canonical SoT name;
/// `IMPLEMENTATION_PLAN.md` is this project's scaffolding rename. First match wins.
const PLAN_FILE_NAMES: [&str; 2] = ["MVP_TASKS.md", "IMPLEMENTATION_PLAN.md"];

/// Detect scaffolding / plan / Brain signals at `path` (a project root). Pure presence checks — a
/// bare directory → all signals absent (`WorkflowDetection::default()`), never an error.
pub fn detect_workflow(path: &Path) -> WorkflowDetection {
    let workflow_pack = path.join(".scaffolding").join("manifest.json").is_file();
    let cc_crew = path.join(".claude").is_dir();
    let plan_file = PLAN_FILE_NAMES
        .iter()
        .map(|name| path.join(name))
        .find(|candidate| candidate.is_file());
    // `.brain` is a provisional MVP marker; the real project↔Brain association lands in Phase 8.
    // Presence-only: `.exists()` (not `is_file`/`is_dir`) — a `.brain` config file OR a `.brain/`
    // directory both signal the association, so any node type counts.
    let brain = path.join(".brain").exists();

    WorkflowDetection {
        workflow_pack,
        cc_crew,
        plan_file,
        brain,
    }
}
