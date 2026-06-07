import { useState } from "react";
import type { AuditEventRow } from "../contracts/index";
import { deriveActivityFeed } from "./derive";

/**
 * Activity dock (§11): a collapsed status-bar summary that expands into a
 * project-filtered event timeline bound to the AuditTrail projection (→ "Full
 * audit" lands later). Collapsed ↔ expanded is state-driven.
 */
export function ActivityDock({ events }: { events: AuditEventRow[] }) {
  const [expanded, setExpanded] = useState(false);
  // TODO(6.2): scope to the selected project once project-selection state lands;
  // 6.1b has no selection yet, so the dock shows the global feed.
  const feed = deriveActivityFeed(events);

  return (
    <div className="activity-dock" aria-label="Activity dock">
      <button
        type="button"
        className="activity-dock__toggle"
        aria-expanded={expanded}
        onClick={() => setExpanded((v) => !v)}
      >
        Activity {expanded ? "▾" : "▸"} ({feed.length})
      </button>
      {expanded ? (
        <ul className="activity-timeline" data-testid="activity-timeline">
          {feed.map((event) => (
            <li key={event.event_id} className="activity-timeline__item">
              <span className="activity-timeline__type">{event.event_type}</span>
              {event.summary ? (
                <span className="activity-timeline__summary">
                  {" "}
                  — {event.summary}
                </span>
              ) : null}
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
