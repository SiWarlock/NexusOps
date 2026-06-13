// A deterministic fixture terminal stream — frozen §6.4 `terminal_output` frames
// (base64 raw PTY bytes), monotonic `seq` from 0. The §14 dev/test seam the
// MockGatewayPort yields; the real UdsGatewayPort demuxes the live
// `ServerFrame.terminal_output` (P4/transport). Output ONLY (the §17 exit is a
// daemon event→projection, not a terminal-channel frame).
import type { TerminalOutputFrame } from "../contracts/index";

const line = (text: string): string => btoa(text);

export const terminalOutputFixture: TerminalOutputFrame[] = [
  { frame_type: "terminal_output", terminal_id: "term_fixture_1", seq: 0, data: line("$ cargo build\r\n") },
  { frame_type: "terminal_output", terminal_id: "term_fixture_1", seq: 1, data: line("   Compiling nexusops-daemon v0.1.0\r\n") },
  { frame_type: "terminal_output", terminal_id: "term_fixture_1", seq: 2, data: line("    Finished dev [unoptimized + debuginfo] in 4.21s\r\n") },
  { frame_type: "terminal_output", terminal_id: "term_fixture_1", seq: 3, data: line("$ ") },
];
