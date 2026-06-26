import { useRef, useState } from "react";
import { SlidersHorizontal } from "lucide-react";
import { Button } from "../../design-system/kit";
import type { GatewayPort } from "../../gateway-client/types";
import type { ProfileRow } from "../../contracts/index";
import { useCanSubmitIntent } from "../../connection/read-only";
import {
  buildProfileChangeActionRequest,
  isProfileChangeEnabled,
} from "../../intent/profile-change-request";
import { isEndedSession } from "../terminal/session-lifecycle";

/**
 * The per-session "Change profile" control (WAVE-1 W1-C-a) — rebinds a session's execution profile via a
 * generic `session.profile_change` `submit_action`. UNLIKE Kill/Launch (risk-0 auto-execute), this is
 * **risk-2 → APPROVAL-GATED** (the §15 #8 no-silent-account-hop checkpoint): the submit returns
 * `awaiting_approval` and surfaces in the live ApprovalQueue — the user approves via the existing card.
 * This control only SUBMITS; the approval + execution + the SessionProfileChanged result ride the existing
 * infra (the PR-mutations precedent). So the on-ok notice is NON-OPTIMISTIC "submitted for approval" (the
 * daemon-reported status, NEVER a fabricated "changed").
 *
 * The picker fetches `get_execution_profiles` ON-DEMAND (when opened, not eagerly), pre-selects the
 * is_default profile, shows the session's CURRENT profile, and marks `has_credential:false` profiles
 * "needs credential" (a disabled, non-selectable option — §2.8/§15 #8). Gated fail-safe + defense-in-depth:
 * renders ONLY for a non-terminal session; disabled unless `canSubmitIntent` (forbidden #6) AND
 * `mutationsEnabled` (the live L2 seam) AND `enabledProfileChange` (the default-OFF gate — HELD until a
 * USER cat-1 sign-off; defense-in-depth to the daemon risk-2 approval, the enabledSessionKill mirror).
 * A §6.4 rejection → the VERBATIM code; a transport fault → honest degrade (LESSON §16/§22, `instanceof Error`).
 */
export function ProfileChangeControl({
  gateway,
  sessionId,
  status,
  currentProfileId,
}: {
  gateway: GatewayPort;
  sessionId: string;
  status: string;
  /** The session's current profile (SessionRowVM.executionProfileId — `string | undefined`, never null). */
  currentProfileId?: string;
}) {
  const canSubmit = useCanSubmitIntent();
  const [open, setOpen] = useState(false);
  const [profiles, setProfiles] = useState<ProfileRow[] | null>(null);
  const [selected, setSelected] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  // Synchronous in-flight guard: `submitting` only disables AFTER the next render, so a rapid double-fire
  // could otherwise submit TWO profile-change intents before the disable commits (the ui-083 review pin).
  const inFlight = useRef(false);

  // A terminal session can't change profile → render nothing (no dead control, §11.6). Hooks run first.
  if (isEndedSession(status)) return null;

  const gateEnabled =
    canSubmit && gateway.mutationsEnabled && isProfileChangeEnabled(gateway);

  // A profile is submittable ONLY if the SELECTED one is credentialed (a needs-credential profile is
  // non-selectable, §15 #8) — covers the empty selection AND a programmatically-set disabled option.
  const selectedIsCredentialed = (profiles ?? []).some(
    (p) => p.execution_profile_id === selected && p.has_credential,
  );

  async function onOpen() {
    if (!gateEnabled) return;
    setOpen(true);
    setNotice(null);
    try {
      const result = await gateway.get_execution_profiles();
      setProfiles(result.profiles);
      // pre-select the default (credentialed) profile → else the first credentialed. NEVER fall back to an
      // un-credentialed profile (those are non-selectable, §15 #8) — if none are credentialed, leave "".
      const pick =
        result.profiles.find((p) => p.is_default && p.has_credential) ??
        result.profiles.find((p) => p.has_credential);
      setSelected(pick?.execution_profile_id ?? "");
    } catch (e) {
      setProfiles([]);
      setNotice(
        e instanceof Error
          ? "Couldn't load profiles — the daemon is unavailable."
          : `Couldn't load profiles (${(e as { code?: string }).code ?? "unknown"}).`,
      );
    }
  }

  async function onSubmit() {
    if (!gateEnabled || inFlight.current || !selectedIsCredentialed) return;
    inFlight.current = true;
    setSubmitting(true);
    setNotice(null);
    try {
      const ack = await gateway.submit_action(
        buildProfileChangeActionRequest(
          { session_id: sessionId, execution_profile_id: selected },
          new Date().toISOString(),
        ),
      );
      // NON-optimistic: risk-2 ENTERS the approval flow — surface the daemon-reported status, never "changed".
      setNotice(
        `Submitted for approval — approve it in the live queue (status: ${ack.status}).`,
      );
      setOpen(false);
    } catch (e) {
      // verbatim §6.4 code (PLAIN data) vs an Error (a real transport/host fault) — classify by
      // `instanceof Error` (LESSON §22), never collapse a wire code.
      if (e instanceof Error) {
        setNotice("Couldn't submit the change — the daemon is unavailable.");
      } else {
        const code = (e as { code?: string }).code ?? "unknown";
        setNotice(`Couldn't submit the change (${code}).`);
      }
    } finally {
      setSubmitting(false);
      inFlight.current = false;
    }
  }

  function optionLabel(p: ProfileRow): string {
    const parts = [p.execution_profile_id];
    if (p.execution_profile_id === currentProfileId) parts.push("(current)");
    if (p.is_default) parts.push("(default)");
    if (!p.has_credential) parts.push("(needs credential)");
    return parts.join(" ");
  }

  return (
    <span
      className="sessions__profile-change"
      style={{ display: "inline-flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}
    >
      {!open ? (
        <span
          title={
            gateEnabled
              ? "Change this session's execution profile (needs approval)"
              : "Connect to the daemon to change the profile (held until sign-off)"
          }
        >
          <Button
            variant="ghost"
            size="sm"
            icon={<SlidersHorizontal size={12} />}
            disabled={!gateEnabled}
            onClick={onOpen}
          >
            Change profile
          </Button>
        </span>
      ) : (
        <span style={{ display: "inline-flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
          <span style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-faint)" }}>
            Current profile: {currentProfileId ?? "—"}
          </span>
          <label htmlFor={`profile-select-${sessionId}`} className="sr-only">
            Choose execution profile
          </label>
          <select
            id={`profile-select-${sessionId}`}
            value={selected}
            onChange={(e) => setSelected(e.target.value)}
            disabled={submitting}
            style={{
              font: "var(--fs-meta) var(--font-sans)",
              padding: "4px 6px",
              borderRadius: "var(--r-2)",
              border: "1px solid var(--border-default)",
              background: "var(--surface-input)",
            }}
          >
            {(profiles ?? []).map((p) => (
              // a needs-credential profile is disabled (not selectable) + annotated — §2.8/§15 #8.
              <option
                key={p.execution_profile_id}
                value={p.execution_profile_id}
                disabled={!p.has_credential}
              >
                {optionLabel(p)}
              </option>
            ))}
          </select>
          <Button
            variant="primary"
            size="sm"
            disabled={!gateEnabled || submitting || !selectedIsCredentialed}
            onClick={onSubmit}
          >
            Submit profile change
          </Button>
          <Button
            variant="ghost"
            size="sm"
            disabled={submitting}
            onClick={() => {
              setOpen(false);
              setNotice(null);
            }}
          >
            Cancel
          </Button>
        </span>
      )}
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
    </span>
  );
}
