import { describe, it, expect } from "vitest";
import { groupForCommandCenter, type CommandItem } from "./group";

describe("groupForCommandCenter", () => {
  it("groups_items_into_triage_buckets", () => {
    const items: CommandItem[] = [
      { id: "n", label: "needs", machine: "Session", status: "waiting_on_human_input" }, // 5 → needs
      { id: "w", label: "work", machine: "Session", status: "running_command" }, // 2 → working
      { id: "s", label: "settled", machine: "Session", status: "idle" }, // 0 → settled
      { id: "f", label: "failed", machine: "Session", status: "failed" }, // 4 → needs
    ];
    const g = groupForCommandCenter(items);
    expect(g.needsAttention.map((i) => i.id).toSorted()).toEqual(["f", "n"]);
    expect(g.working.map((i) => i.id)).toEqual(["w"]);
    expect(g.settled.map((i) => i.id)).toEqual(["s"]);
  });

  it("changes_ready_extracted_as_its_own_group", () => {
    const items: CommandItem[] = [
      { id: "cr1", label: "cr", machine: "Session", status: "changes_ready" }, // rank 4
      { id: "cr2", label: "cr2", machine: "Task", status: "changes_ready" }, // rank 4
      { id: "f", label: "failed", machine: "Session", status: "failed" }, // rank 4 (needs)
    ];
    const g = groupForCommandCenter(items);
    expect(g.changesReady.map((i) => i.id).toSorted()).toEqual(["cr1", "cr2"]);
    // changes_ready is extracted — NOT duplicated into needs-attention
    expect(g.needsAttention.map((i) => i.id)).toEqual(["f"]);
  });

  it("within_group_sorted_by_attention", () => {
    const items: CommandItem[] = [
      { id: "a", label: "a", machine: "Session", status: "failed" }, // 4
      { id: "b", label: "b", machine: "Session", status: "waiting_on_human_input" }, // 5
    ];
    const g = groupForCommandCenter(items);
    expect(g.needsAttention.map((i) => i.id)).toEqual(["b", "a"]); // higher rank first
  });
});
