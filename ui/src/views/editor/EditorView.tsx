import { useState } from "react";
import {
  Check,
  ChevronDown,
  ChevronRight,
  FileCode,
  Folder,
  FolderGit2,
  X,
} from "lucide-react";
import { Badge, Button, HarnessBadge, RiskBadge } from "../../design-system/kit";
import { Eyebrow } from "../cockpit";
import {
  editorFilesFixture,
  editorTreeFixture,
  type TreeNodeFx,
} from "../display-fixtures";

/* Lightweight TS tokenizer for cosmetic highlighting (the prototype's `hi`). */
const KW =
  /\b(import|from|export|async|function|const|let|return|if|else|await|type|interface|new|class)\b/;
const TYPE = /\b([A-Z][A-Za-z0-9]+)\b/;

function highlight(line: string): { c: string; t: string }[] {
  if (line.trim().startsWith("//")) return [{ c: "var(--text-faint)", t: line }];
  const out: { c: string; t: string }[] = [];
  const parts = line.split(/('[^']*')/g);
  for (const p of parts) {
    if (p.startsWith("'") && p.endsWith("'")) {
      out.push({ c: "var(--success-ink)", t: p });
      continue;
    }
    const re = new RegExp(`${KW.source}|${TYPE.source}`, "g");
    let last = 0;
    let m: RegExpExecArray | null;
    while ((m = re.exec(p))) {
      if (m.index > last) out.push({ c: "var(--text-secondary)", t: p.slice(last, m.index) });
      out.push({ c: KW.test(m[0]) ? "var(--accent-ink)" : "var(--brain-ink)", t: m[0] });
      last = m.index + m[0].length;
    }
    if (last < p.length) out.push({ c: "var(--text-secondary)", t: p.slice(last) });
  }
  return out;
}

function TreeRow({
  node,
  activeFile,
  onOpen,
}: {
  node: TreeNodeFx;
  activeFile: string;
  onOpen: (name: string) => void;
}) {
  const isFile = node.type === "file";
  const active = isFile && node.name === activeFile;
  const gitColor =
    node.git === "M" ? "var(--caution-ink)" : node.git === "A" ? "var(--success-ink)" : "var(--danger-ink)";
  const inner = (
    <>
      <span aria-hidden="true" style={{ display: "inline-flex", color: "var(--text-faint)", flex: "none" }}>
        {isFile ? <FileCode size={13} /> : node.open ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
      </span>
      {!isFile && !node.open ? (
        <span aria-hidden="true" style={{ display: "inline-flex", color: "var(--text-faint)", marginLeft: -2 }}>
          <Folder size={13} />
        </span>
      ) : null}
      <span
        style={{
          flex: 1,
          font: `${isFile ? "var(--fw-regular)" : "var(--fw-medium)"} var(--fs-meta) var(--font-mono)`,
          color: active ? "var(--text-primary)" : "var(--text-secondary)",
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
          textAlign: "left",
        }}
      >
        {node.name}
      </span>
      {node.agent ? (
        <span
          title="agent editing"
          style={{
            width: 6,
            height: 6,
            borderRadius: 999,
            background: "var(--accent-solid)",
            animation: "cp-live-pulse 1.6s var(--ease-inout) infinite",
            flex: "none",
          }}
        />
      ) : null}
      {node.git ? (
        <span style={{ font: "var(--fw-semibold) var(--fs-micro) var(--font-mono)", color: gitColor }}>
          {node.git}
        </span>
      ) : null}
    </>
  );
  const rowStyle = {
    display: "flex",
    alignItems: "center",
    gap: 6,
    height: 24,
    width: "100%",
    paddingLeft: 8 + node.depth * 13,
    paddingRight: 8,
    borderRadius: "var(--r-1)",
    border: "none",
    background: active ? "var(--surface-active)" : "transparent",
  } as const;
  return isFile ? (
    <button type="button" onClick={() => onOpen(node.name)} style={{ ...rowStyle, cursor: "pointer" }}>
      {inner}
    </button>
  ) : (
    <div style={rowStyle}>{inner}</div>
  );
}

/**
 * The Editor / IDE view (ported from kit-views3.jsx EditorView): explorer tree,
 * tab bar (dirty dots, close, live-edit pill), the code pane with diff gutter
 * marks + the inline agent-edit cursor, and the Agent-activity / Problems
 * bottom panel. DISPLAY-ONLY over a provisional fixture — the worktree
 * file-access contract is daemon-gated (flagged); Approve-edit is disabled,
 * not faked (§11.6). Tabs/tree selection are pure local UI state.
 */
export function EditorView() {
  const files = editorFilesFixture;
  const [active, setActive] = useState("review.ts");
  const [openTabs, setOpenTabs] = useState(["review.ts", "risk.ts", "gateway.test.ts"]);
  const file = files[active] ?? files["review.ts"]!;
  const agentEdit = file.agentEdit;
  const problems = file.problems;
  const markBg: Record<string, string> = {
    add: "var(--diff-add-bg)",
    del: "var(--diff-del-bg)",
    ctx: "transparent",
  };
  const markGutter: Record<string, string> = {
    add: "var(--diff-add-gutter)",
    del: "var(--diff-del-gutter)",
  };

  const openFile = (name: string) => {
    if (!files[name]) return;
    setActive(name);
    setOpenTabs((o) => (o.includes(name) ? o : [...o, name]));
  };
  const closeTab = (name: string) => {
    setOpenTabs((o) => {
      const next = o.filter((n) => n !== name);
      if (active === name && next.length) setActive(next[next.length - 1]!);
      return next.length ? next : o;
    });
  };

  return (
    <div
      aria-label="Editor"
      style={{
        display: "grid",
        gridTemplateColumns: "210px 1fr",
        gridTemplateRows: "1fr",
        height: "100%",
        background: "var(--surface-canvas)",
        minHeight: 0,
      }}
    >
      {/* Explorer */}
      <aside
        style={{
          borderRight: "1px solid var(--border-default)",
          background: "var(--surface-panel)",
          overflowY: "auto",
          padding: "10px 6px",
        }}
      >
        <Eyebrow style={{ padding: "0 8px 8px", display: "flex", alignItems: "center", gap: 6 }}>
          <FolderGit2 size={12} aria-hidden="true" /> auth-service
        </Eyebrow>
        <div style={{ padding: "0 8px 8px" }}>
          <Badge mono size="xs" style={{ color: "var(--text-faint)" }}>
            display fixture — worktree FS pending
          </Badge>
        </div>
        {editorTreeFixture.map((node, i) => (
          <TreeRow key={i} node={node} activeFile={active} onOpen={openFile} />
        ))}
      </aside>

      {/* Editor column */}
      <div style={{ display: "grid", gridTemplateRows: "34px 1fr 150px", minWidth: 0, minHeight: 0 }}>
        {/* Tab bar */}
        <div
          style={{
            display: "flex",
            alignItems: "stretch",
            background: "var(--surface-sunken)",
            borderBottom: "1px solid var(--border-default)",
            overflowX: "auto",
          }}
        >
          {openTabs.map((name) => {
            const f = files[name];
            if (!f) return null;
            const on = name === active;
            return (
              <div
                key={name}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 7,
                  padding: "0 10px 0 12px",
                  borderRight: "1px solid var(--border-subtle)",
                  background: on ? "var(--surface-canvas)" : "transparent",
                  color: on ? "var(--text-primary)" : "var(--text-muted)",
                  boxShadow: on ? "inset 0 2px 0 var(--accent-solid)" : "none",
                  font: "var(--fs-meta) var(--font-mono)",
                }}
              >
                <button
                  type="button"
                  onClick={() => setActive(name)}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 7,
                    border: "none",
                    background: "transparent",
                    color: "inherit",
                    font: "inherit",
                    cursor: "pointer",
                    padding: 0,
                  }}
                >
                  <FileCode
                    size={12}
                    aria-hidden="true"
                    style={{ color: f.agent ? "var(--accent-ink)" : "var(--text-faint)" }}
                  />
                  {name}
                </button>
                {f.dirty ? (
                  <span
                    title="unsaved agent changes"
                    style={{ width: 6, height: 6, borderRadius: 999, background: "var(--caution-solid)" }}
                  />
                ) : (
                  <button
                    type="button"
                    onClick={() => closeTab(name)}
                    style={{
                      display: "inline-flex",
                      borderRadius: 3,
                      border: "none",
                      background: "transparent",
                      cursor: "pointer",
                      padding: 0,
                    }}
                  >
                    <X size={11} aria-hidden="true" style={{ color: "var(--text-faint)" }} />
                    <span className="sr-only">Close {name}</span>
                  </button>
                )}
              </div>
            );
          })}
          <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 4, padding: "0 8px" }}>
            {file.agent ? (
              <>
                <HarnessBadge harness="claude-code" showLabel={false} />
                <span style={{ font: "var(--fs-micro) var(--font-mono)", color: "var(--accent-ink)" }}>
                  live edit
                </span>
              </>
            ) : null}
          </div>
        </div>

        {/* Code area */}
        <div
          style={{
            overflow: "auto",
            background: "var(--surface-canvas)",
            position: "relative",
            fontFamily: "var(--font-mono)",
            fontSize: 13,
          }}
        >
          {file.lines.map((line, i) => {
            const mark = file.marks[i];
            const isAgent = agentEdit?.line === i;
            return (
              <div
                key={i}
                style={{
                  display: "flex",
                  alignItems: "stretch",
                  background: mark ? markBg[mark] : isAgent ? "var(--accent-surface)" : "transparent",
                  minHeight: 22,
                  lineHeight: "22px",
                  position: "relative",
                }}
              >
                <span
                  style={{
                    width: 44,
                    flex: "none",
                    textAlign: "right",
                    padding: "0 10px 0 0",
                    color: "var(--text-faint)",
                    userSelect: "none",
                    boxShadow: mark && markGutter[mark] ? `inset -2px 0 0 ${markGutter[mark]}` : "none",
                  }}
                >
                  {i + 1}
                </span>
                <span style={{ paddingLeft: 10, whiteSpace: "pre", flex: 1, minWidth: 0 }}>
                  {highlight(line).map((tk, j) => (
                    <span key={j} style={{ color: tk.c }}>
                      {tk.t}
                    </span>
                  ))}
                  {isAgent ? (
                    <span
                      aria-hidden="true"
                      style={{
                        display: "inline-block",
                        width: 2,
                        height: 15,
                        marginLeft: 1,
                        background: "var(--accent-solid)",
                        verticalAlign: "middle",
                        animation: "cp-live-pulse 1.1s steps(1) infinite",
                      }}
                    />
                  ) : null}
                </span>
                {isAgent && agentEdit ? (
                  <span
                    style={{
                      position: "absolute",
                      right: 8,
                      top: 1,
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 5,
                      height: 18,
                      padding: "0 7px",
                      borderRadius: "var(--r-1)",
                      background: "var(--accent-solid)",
                      color: "var(--accent-on-solid)",
                      font: "var(--fw-medium) var(--fs-micro)/1 var(--font-sans)",
                    }}
                  >
                    <span
                      aria-hidden="true"
                      style={{
                        width: 5,
                        height: 5,
                        borderRadius: 999,
                        background: "currentColor",
                        animation: "cp-live-pulse 1.6s var(--ease-inout) infinite",
                      }}
                    />
                    {agentEdit.who}
                  </span>
                ) : null}
              </div>
            );
          })}
        </div>

        {/* Bottom panel */}
        <div
          style={{
            borderTop: "1px solid var(--border-default)",
            background: "var(--surface-sunken)",
            display: "flex",
            flexDirection: "column",
            minHeight: 0,
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 14,
              padding: "0 12px",
              height: 30,
              borderBottom: "1px solid var(--border-subtle)",
            }}
          >
            <span
              style={{
                font: "var(--fw-semibold) var(--fs-micro) var(--font-sans)",
                letterSpacing: "var(--tracking-caps)",
                textTransform: "uppercase",
                color: "var(--text-primary)",
                borderBottom: "2px solid var(--accent-solid)",
                height: 30,
                display: "flex",
                alignItems: "center",
              }}
            >
              {agentEdit ? "Agent activity" : "Problems"}
            </span>
            <span
              style={{
                font: "var(--fw-semibold) var(--fs-micro) var(--font-sans)",
                letterSpacing: "var(--tracking-caps)",
                textTransform: "uppercase",
                color: "var(--text-muted)",
                display: "inline-flex",
                alignItems: "center",
                gap: 5,
              }}
            >
              Problems{" "}
              <Badge mono size="xs" style={{ color: problems.length ? "var(--warning-ink)" : "var(--text-faint)" }}>
                {problems.length}
              </Badge>
            </span>
            <span style={{ marginLeft: "auto", font: "var(--fs-meta) var(--font-mono)", color: "var(--text-faint)" }}>
              {file.path}/{active}
            </span>
            {agentEdit ? (
              <>
                <RiskBadge level="medium" />
                <span title="Edit approval arrives with the Gateway overlay (intent seam — daemon-gated)">
                  <Button variant="secondary" size="sm" icon={<Check size={13} />} disabled>
                    Approve edit
                  </Button>
                </span>
              </>
            ) : null}
          </div>
          <div
            style={{
              overflowY: "auto",
              padding: "8px 12px",
              font: "var(--fs-meta)/1.7 var(--font-mono)",
              color: "var(--text-secondary)",
            }}
          >
            {agentEdit ? (
              <>
                <div>
                  <span style={{ color: "var(--accent-ink)" }}>{agentEdit.who}</span> · {agentEdit.note}
                </div>
                <div style={{ color: "var(--text-faint)" }}>
                  ↳ proposing edits in {file.path}/{active}
                </div>
                {problems[0] ? (
                  <div style={{ color: "var(--warning-ink)" }}>
                    ⚠ {problems[0].text} <span style={{ color: "var(--text-faint)" }}>({problems[0].at})</span>
                  </div>
                ) : null}
                <div style={{ color: "var(--text-faint)" }}>awaiting approval to apply edit + run tests…</div>
              </>
            ) : problems.length ? (
              problems.map((p, i) => (
                <div key={i} style={{ color: p.sev === "fail" ? "var(--danger-ink)" : "var(--warning-ink)" }}>
                  {p.sev === "fail" ? "✕" : "⚠"} {p.text}{" "}
                  <span style={{ color: "var(--text-faint)" }}>({p.at})</span>
                </div>
              ))
            ) : (
              <div style={{ color: "var(--text-faint)" }}>No problems in this file.</div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
