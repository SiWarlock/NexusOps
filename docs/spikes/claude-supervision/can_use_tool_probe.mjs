// OQ-HARN-SPIKE-7 — can_use_tool coverage probe (MVP task 0.1). SCAFFOLDING.
//
// Empirically re-confirms the ARCHITECTURE §9.1 mutation-coverage matrix on the
// PINNED Claude stack: which tool categories actually reach the `canUseTool`
// callback under `default` permission mode (O-13: the only mode NexusOps ships).
//
// NOT RUN at authoring time: the Agent SDK was not installed and no auth was set,
// and running it burns Claude quota (esp. after the 2026-06-15 SDK credit-pool
// split). The SDK call shape is marked TODO-VERIFY — confirm against the pinned
// `@anthropic-ai/claude-agent-sdk` before trusting the output. Fold this run in
// with the ≥2026-06-15 drain checklist (both need the SDK + auth).
//
// Setup:  npm i @anthropic-ai/claude-agent-sdk@<PINNED>   (+ auth: ANTHROPIC_API_KEY or `claude setup-token`)
// Run:    node can_use_tool_probe.mjs
// Expect (per §9.1): direct bash/Write/Edit + MCP(direct) = INTERCEPTED;
//                    foreground Task subagent = NOT guaranteed;
//                    background Task subagent = BYPASSED (#27203, won't-fix).

// TODO-VERIFY: import name/shape vs the pinned SDK.
import { query } from "@anthropic-ai/claude-agent-sdk";

const seen = new Map(); // toolName -> intercepted count
const mark = (t) => seen.set(t, (seen.get(t) ?? 0) + 1);

const prompts = [
  "Run the bash command: echo direct-bash",
  "Write a file /tmp/nexus_probe.txt with the text hello",
  "Use the Task tool to launch a FOREGROUND subagent that runs: echo fg-subagent",
  "Use the Task tool with run_in_background=true to launch a BACKGROUND subagent that runs: echo bg-subagent",
];

for (const prompt of prompts) {
  try {
    const run = query({
      prompt,
      options: {
        permissionMode: "default", // O-13: NexusOps ships default ONLY
        // TODO-VERIFY: callback name + return shape vs pinned SDK
        canUseTool: async (toolName, input /*, ctx */) => {
          mark(toolName);
          console.log(`  INTERCEPTED  ${toolName}  ${JSON.stringify(input).slice(0, 80)}`);
          return { behavior: "allow", updatedInput: input };
        },
      },
    });
    for await (const _msg of run) {
      /* drain the message stream; status could also be derived here (§9.1) */
    }
  } catch (e) {
    console.log(`  (prompt errored: ${e?.message ?? e})`);
  }
}

console.log("\n# canUseTool interception tally (default mode):");
for (const [t, n] of seen) console.log(`  ${t}: ${n}`);
console.log(
  "\n# Cross-check vs §9.1: bash/Write/Edit + MCP(direct) should appear;\n" +
    "# a BACKGROUND subagent's inner tool call should be ABSENT (= #27203 bypass confirmed)."
);
