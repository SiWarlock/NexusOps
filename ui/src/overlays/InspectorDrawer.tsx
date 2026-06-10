import { ArrowRight, Bot, FolderGit2, GitPullRequest, X } from "lucide-react";
import { Button, IconButton } from "../design-system/kit";
import { StatusPill } from "../status/StatusPill";
import type { GraphNode } from "../views/graph/model";
import { sessionDisplayFixture, contextForSession } from "../shell/display-meta";
import type { UsageRow } from "../contracts/index";
import { Eyebrow } from "../views/cockpit";
import { Overlay } from "./Overlay";

/**
 * The graph-node Inspector drawer (ported from kit-overlays.jsx
 * InspectorDrawer): identity header, the REAL (machine,status) pill, a detail
 * grid (frozen-row fields + display side-map ownership + REAL usage ctx — the
 * §9.1 "unknown" rule holds), and the Open jump to the node's surface (live
 * navigation).
 */
export function InspectorDrawer({
  node,
  usage,
  onClose,
  onOpen,
}: {
  node: GraphNode;
  usage: UsageRow[];
  onClose: () => void;
  onOpen: (node: GraphNode) => void;
}) {
  const rawId = node.id.split(":")[1] ?? "";
  const display = node.type === "session" ? sessionDisplayFixture[rawId] : undefined;
  const ctx = node.type === "session" ? contextForSession(usage, rawId) : null;

  const details: [string, string][] = [];
  if (display?.harness) details.push(["Harness", display.harness]);
  if (display?.profile) details.push(["Profile", display.profile]);
  if (display?.branch) details.push(["Branch", display.branch]);
  if (display?.worktree) details.push(["Worktree", display.worktree]);
  if (ctx) details.push(["Context", ctx.pct === null ? "unknown" : `${ctx.pct}%`]);
  if (display?.activity) details.push(["Activity", display.activity]);

  const icon =
    node.type === "session" ? <Bot size={15} /> : node.type === "pull_request" ? <GitPullRequest size={15} /> : <FolderGit2 size={15} />;
  const openLabel =
    node.type === "session"
      ? display?.team
        ? "Open team view"
        : "Open terminal"
      : node.type === "pull_request"
        ? "Open review"
        : "Open project";

  return (
    <Overlay onClose={onClose} align="right" width={340} label="Inspector">
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          height: "100%",
          background: "var(--surface-panel)",
          borderLeft: "1px solid var(--border-strong)",
        }}
        data-testid="inspector-drawer"
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 10,
            padding: "12px 14px",
            borderBottom: "1px solid var(--border-default)",
          }}
        >
          <span
            aria-hidden="true"
            style={{
              width: 28,
              height: 28,
              flex: "none",
              borderRadius: "var(--r-2)",
              background: "var(--surface-active)",
              color: "var(--text-secondary)",
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            {icon}
          </span>
          <div style={{ minWidth: 0, flex: 1 }}>
            <div
              style={{
                font: "var(--fw-semibold) var(--fs-body) var(--font-sans)",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {node.label}
            </div>
            <div
              style={{
                font: "var(--fs-micro) var(--font-mono)",
                color: "var(--text-faint)",
                textTransform: "uppercase",
                letterSpacing: "var(--tracking-caps)",
              }}
            >
              Inspector · {node.type.replace("_", " ")}
            </div>
          </div>
          <IconButton label="Close" onClick={onClose}>
            <X size={15} />
          </IconButton>
        </div>
        {node.machine && node.status ? (
          <div
            style={{
              padding: "12px 14px",
              borderBottom: "1px solid var(--border-subtle)",
              display: "flex",
              alignItems: "center",
              gap: 8,
            }}
          >
            <StatusPill machine={node.machine} status={node.status} />
          </div>
        ) : null}
        <div style={{ flex: 1, overflowY: "auto", padding: "12px 14px" }}>
          <Eyebrow style={{ marginBottom: 8 }}>Details</Eyebrow>
          {details.length === 0 ? (
            <div style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-faint)" }}>
              Detail fields arrive with the daemon projection enrichment.
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column" }}>
              {details.map(([k, v]) => (
                <div
                  key={k}
                  style={{
                    display: "flex",
                    alignItems: "baseline",
                    gap: 12,
                    padding: "7px 0",
                    borderBottom: "1px solid var(--border-subtle)",
                  }}
                >
                  <span style={{ flex: "none", width: 84, font: "var(--fs-meta) var(--font-sans)", color: "var(--text-faint)" }}>
                    {k}
                  </span>
                  <span
                    style={{
                      font: "var(--fs-meta) var(--font-mono)",
                      color: "var(--text-secondary)",
                      textAlign: "right",
                      marginLeft: "auto",
                      overflowWrap: "anywhere",
                    }}
                  >
                    {v}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
        <div style={{ padding: "12px 14px", borderTop: "1px solid var(--border-default)", display: "flex", gap: 8 }}>
          <Button
            variant="primary"
            size="md"
            full
            icon={<ArrowRight size={14} />}
            onClick={() => onOpen(node)}
          >
            {openLabel}
          </Button>
        </div>
      </div>
    </Overlay>
  );
}
