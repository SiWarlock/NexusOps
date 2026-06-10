import { useState } from "react";
import {
  Brain,
  CheckCheck,
  ChevronDown,
  ChevronRight,
  FileText,
  ListChecks,
  Play,
  UsersRound,
} from "lucide-react";
import { Badge, Button, DisplayStatusPill, MetaChip } from "../../design-system/kit";
import { Eyebrow } from "../cockpit";
import { planFixture, type PlanPhaseFx, type PlanTaskFx } from "../display-fixtures";

// task status → kit pill kind + label (display fixture states, no frozen machine).
const TASK_STATUS = {
  completed: { pill: "completed", label: "Done" },
  "in-progress": { pill: "running", label: "In progress" },
  ready: { pill: "idle", label: "Ready" },
  todo: { pill: "idle", label: "Todo" },
  backlog: { pill: "archived", label: "Backlog" },
} as const;
const PHASE_STATUS = { completed: "completed", active: "running", backlog: "archived" } as const;

function PlanTaskRow({ task }: { task: PlanTaskFx }) {
  const st = TASK_STATUS[task.status] ?? TASK_STATUS.todo;
  const dispatchable = task.status !== "completed";
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        padding: "9px 10px",
        borderRadius: "var(--r-2)",
        border: "1px solid var(--border-subtle)",
        background: "var(--surface-panel)",
      }}
    >
      <span style={{ font: "var(--fs-meta) var(--font-mono)", color: "var(--text-faint)", width: 30, flex: "none" }}>
        {task.id}
      </span>
      <DisplayStatusPill status={st.pill} size="xs" label={st.label} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            font: "var(--fw-medium) var(--fs-label)/1.3 var(--font-sans)",
            color: "var(--text-primary)",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {task.title}
        </div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            marginTop: 3,
            font: "var(--fs-micro) var(--font-mono)",
            color: "var(--text-faint)",
          }}
        >
          <span title="acceptance criteria" style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
            <CheckCheck size={11} aria-hidden="true" /> {task.ac} AC
          </span>
          {task.files > 0 ? <span title="files">· {task.files} files</span> : null}
          {task.anchors.map((a) => (
            <span key={a} style={{ color: "var(--brain-ink)" }}>
              · ⚓ {a.split("#")[1] ?? a}
            </span>
          ))}
          {task.session ? <span style={{ color: "var(--live-ink)" }}>· ● in session</span> : null}
          {task.team ? <span style={{ color: "var(--teal-ink)" }}>· team</span> : null}
        </div>
      </div>
      {task.task ? (
        <MetaChip tone={task.task.tone} mono={false}>
          {task.task.id}
        </MetaChip>
      ) : null}
      {dispatchable ? (
        <span title="Task dispatch arrives with the intent seam (daemon-gated)">
          <Button variant="secondary" size="sm" icon={<Play size={12} />} disabled>
            Start
          </Button>
        </span>
      ) : null}
    </div>
  );
}

function PhaseCard({ phase }: { phase: PlanPhaseFx }) {
  const [open, setOpen] = useState(phase.status !== "completed");
  const tasks = phase.tracks.flatMap((t) => t.tasks);
  const done = tasks.filter((t) => t.status === "completed").length;
  return (
    <div
      style={{
        border: "1px solid var(--border-default)",
        borderRadius: "var(--r-3)",
        overflow: "hidden",
        background: "var(--surface-card)",
      }}
    >
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 9,
          width: "100%",
          padding: "11px 13px",
          border: "none",
          background: phase.status === "active" ? "var(--accent-surface)" : "transparent",
          cursor: "pointer",
          textAlign: "left",
        }}
      >
        <span aria-hidden="true" style={{ display: "inline-flex", color: "var(--text-faint)" }}>
          {open ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
        </span>
        <span
          style={{
            font: "var(--fw-semibold) var(--fs-body) var(--font-sans)",
            color: phase.status === "active" ? "var(--accent-ink)" : "var(--text-primary)",
          }}
        >
          {phase.name}
        </span>
        <DisplayStatusPill status={PHASE_STATUS[phase.status]} size="xs" />
        <span style={{ marginLeft: "auto", font: "var(--fs-meta) var(--font-mono)", color: "var(--text-muted)" }}>
          {done}/{tasks.length}
        </span>
        {phase.status !== "completed" ? (
          <span
            title="Team dispatch arrives with the intent seam (daemon-gated)"
            style={{ display: "inline-flex", alignItems: "center", gap: 4, font: "var(--fs-meta) var(--font-sans)", color: "var(--text-faint)" }}
          >
            <UsersRound size={12} aria-hidden="true" /> Start team
          </span>
        ) : null}
      </button>
      {open ? (
        <div style={{ padding: "4px 10px 10px" }}>
          {phase.tracks.map((tr) => (
            <div key={tr.id} style={{ marginTop: 6 }}>
              <Eyebrow style={{ padding: "4px 6px 7px" }}>{tr.name}</Eyebrow>
              <div style={{ display: "flex", flexDirection: "column", gap: 5 }}>
                {tr.tasks.map((task) => (
                  <PlanTaskRow key={task.id} task={task} />
                ))}
              </div>
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}

/**
 * The Plan view (ported from kit-plan.jsx PlanView): the Phase → Track →
 * PlanTask hierarchy with the parse-source chips and the progress bar.
 * DISPLAY-ONLY over a provisional fixture — the ImplementationPlan projection
 * (workflow-pack parser integration) hasn't landed; dispatch (Start / Start
 * team / Ask Brain) is disabled, not faked (§11.6). Flagged.
 */
export function PlanView() {
  const plan = planFixture;
  const all = plan.phases.flatMap((ph) => ph.tracks.flatMap((t) => t.tasks));
  const done = all.filter((t) => t.status === "completed").length;
  const pct = Math.round((done / all.length) * 100);
  return (
    <div aria-label="Plan" style={{ height: "100%", overflowY: "auto", background: "var(--surface-canvas)" }}>
      <div
        style={{
          position: "sticky",
          top: 0,
          zIndex: 5,
          padding: "14px 16px",
          background: "var(--surface-canvas)",
          borderBottom: "1px solid var(--border-subtle)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <span aria-hidden="true" style={{ color: "var(--text-secondary)", display: "inline-flex" }}>
            <ListChecks size={18} />
          </span>
          <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)" }}>
            Implementation plan
          </h1>
          <MetaChip icon={<FileText size={12} />}>{plan.source}</MetaChip>
          <Badge tone="teal">parser: {plan.parser}</Badge>
          <Badge mono style={{ color: "var(--text-faint)" }}>
            display fixture — plan projection pending
          </Badge>
          <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
            <span title="Brain co-pilot arrives with the sidecar contract (Phase 8)">
              <Button variant="ghost" size="sm" icon={<Brain size={14} />} disabled>
                Ask Brain
              </Button>
            </span>
          </div>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginTop: 11 }}>
          <div
            style={{
              flex: 1,
              maxWidth: 320,
              height: 6,
              borderRadius: 999,
              background: "var(--cap-track)",
              overflow: "hidden",
            }}
          >
            <i style={{ display: "block", height: "100%", width: `${pct}%`, background: "var(--success-solid)" }} />
          </div>
          <span style={{ font: "var(--fs-meta) var(--font-mono)", color: "var(--text-muted)" }}>
            {done}/{all.length} tasks · {pct}%
          </span>
        </div>
      </div>

      <div style={{ padding: "14px 16px", maxWidth: 820, display: "flex", flexDirection: "column", gap: 10 }}>
        {plan.phases.map((ph) => (
          <PhaseCard key={ph.id} phase={ph} />
        ))}
      </div>
    </div>
  );
}
