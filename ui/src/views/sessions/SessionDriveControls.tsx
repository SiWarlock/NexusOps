import { useRef, useState } from "react";
import { Pause, Play, Send } from "lucide-react";
import { Button } from "../../design-system/kit";
import type { GatewayPort } from "../../gateway-client/types";
import type { ActionRequest } from "../../contracts/index";
import { useCanSubmitIntent } from "../../connection/read-only";
import {
  buildSendMessageActionRequest,
  buildPauseActionRequest,
  buildResumeActionRequest,
  isSessionControlEnabled,
  SESSION_SEND_MESSAGE_ACTION_TYPE,
  SESSION_PAUSE_ACTION_TYPE,
  SESSION_RESUME_ACTION_TYPE,
} from "../../intent/session-drive-request";
import { isEndedSession } from "../terminal/session-lifecycle";

/** The UI submission timestamp (a real fact, not an invented consequence) — module-scoped (captures nothing). */
const now = (): string => new Date().toISOString();

/**
 * The session-DRIVE controls (WAVE-1 W1-C-b) — Send message / Pause monitoring / Resume monitoring — for
 * the selected session's detail surface. Each submits a generic `submit_action` intent the daemon
 * APPROVAL-gates (risk≥1 → the live ApprovalQueue, not auto-execute); this control only SUBMITS — the
 * approval + execution ride the existing infra (the profile_change precedent). So the on-ok notice is
 * NON-optimistic "submitted for approval" (the daemon's ActionAck.status, NEVER "sent"/"paused").
 *
 * 🔴 HONEST LABELS (daemon LESSON 71): MVP `session.pause` is SOFT — it gates the cockpit's drive/observe
 * loop, it does NOT OS-suspend the agent (the agent keeps running; its tool calls are still
 * Gateway-intercepted → INV-SEC-1 holds). The labels read "Pause monitoring"/"Resume monitoring" — NEVER
 * "stop"/"suspend". There is no UI-readable paused state (the soft pause isn't projected), so BOTH actions
 * are always available (no stateful toggle).
 *
 * Gated fail-safe + defense-in-depth: renders ONLY for a non-terminal session; each control disabled unless
 * `canSubmitIntent` (forbidden #6) AND `mutationsEnabled` (the live L2 seam) AND
 * `isSessionControlEnabled(gateway, <type>)` (the default-OFF Set gate — HELD until a USER cat-1 sign-off).
 * A §6.4 rejection → the VERBATIM code; a transport fault → honest degrade (LESSON §16/§22).
 */
export function SessionDriveControls({
  gateway,
  sessionId,
  status,
}: {
  gateway: GatewayPort;
  sessionId: string;
  status: string;
}) {
  const canSubmit = useCanSubmitIntent();
  const [message, setMessage] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  // Synchronous in-flight guard: blocks a rapid double-fire (any drive action) before the disable commits.
  const inFlight = useRef(false);

  // A terminal session can't be driven → render nothing (no dead control, §11.6). Hooks run first.
  if (isEndedSession(status)) return null;

  const baseEnabled = canSubmit && gateway.mutationsEnabled;
  const can = (actionType: string): boolean =>
    baseEnabled && isSessionControlEnabled(gateway, actionType);

  async function submit(request: ActionRequest): Promise<boolean> {
    // Gate off the REQUEST's own action_type (single source — no separate param that could mismatch).
    if (!can(request.action_type) || inFlight.current) return false;
    inFlight.current = true;
    setSubmitting(true);
    setNotice(null);
    try {
      const ack = await gateway.submit_action(request);
      // NON-optimistic: risk≥1 ENTERS the approval flow — surface the daemon-reported status, never "sent"/"paused".
      setNotice(
        `Submitted for approval — approve it in the live queue (status: ${ack.status}).`,
      );
      return true;
    } catch (e) {
      // verbatim §6.4 code (PLAIN data) vs an Error (a real transport/host fault) — classify by
      // `instanceof Error` (LESSON §22), never collapse a wire code.
      if (e instanceof Error) {
        setNotice("Couldn't submit — the daemon is unavailable.");
      } else {
        const code = (e as { code?: string }).code ?? "unknown";
        setNotice(`Couldn't submit (${code}).`);
      }
      return false;
    } finally {
      setSubmitting(false);
      inFlight.current = false;
    }
  }

  async function onSend() {
    const trimmed = message.trim();
    if (trimmed === "") return; // block empty — never an empty intent
    const ok = await submit(
      buildSendMessageActionRequest({ session_id: sessionId, message: trimmed }, now()),
    );
    if (ok) setMessage(""); // clear on a successful send
  }

  const onPause = () => void submit(buildPauseActionRequest({ session_id: sessionId }, now()));
  const onResume = () => void submit(buildResumeActionRequest({ session_id: sessionId }, now()));

  const canSend =
    can(SESSION_SEND_MESSAGE_ACTION_TYPE) && !submitting && message.trim() !== "";

  return (
    <div
      className="sessions__drive"
      style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}
    >
      <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
        <label htmlFor={`drive-message-${sessionId}`} className="sr-only">
          Message to send to the session
        </label>
        <input
          id={`drive-message-${sessionId}`}
          type="text"
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          placeholder="Send a message…"
          disabled={!can(SESSION_SEND_MESSAGE_ACTION_TYPE) || submitting}
          style={{
            font: "var(--fs-meta) var(--font-sans)",
            padding: "4px 8px",
            borderRadius: "var(--r-2)",
            border: "1px solid var(--border-default)",
            background: "var(--surface-input)",
            width: 200,
            maxWidth: "36vw",
          }}
        />
        <Button
          variant="secondary"
          size="sm"
          icon={<Send size={12} />}
          disabled={!canSend}
          onClick={onSend}
        >
          Send
        </Button>
      </span>
      <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
        <span title="Pause the cockpit's monitoring of this session — the agent keeps running and stays Gateway-intercepted (this is a soft, monitoring-only pause)">
          <Button
            variant="ghost"
            size="sm"
            icon={<Pause size={12} />}
            disabled={!can(SESSION_PAUSE_ACTION_TYPE) || submitting}
            onClick={onPause}
          >
            Pause monitoring
          </Button>
        </span>
        <span title="Resume the cockpit's monitoring of this session">
          <Button
            variant="ghost"
            size="sm"
            icon={<Play size={12} />}
            disabled={!can(SESSION_RESUME_ACTION_TYPE) || submitting}
            onClick={onResume}
          >
            Resume monitoring
          </Button>
        </span>
      </span>
      {notice ? (
        <span
          role="status"
          style={{
            font: "var(--fs-meta) var(--font-sans)",
            color: "var(--text-secondary)",
            display: "inline-flex",
            alignItems: "center",
            gap: 5,
            maxWidth: 360,
          }}
        >
          {notice}
        </span>
      ) : null}
    </div>
  );
}
