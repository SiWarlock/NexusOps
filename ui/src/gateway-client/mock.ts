// MockGatewayPort — the §14-sanctioned UI test/dev seam.
//
// Implements the §6.1 READ surface against contract-valid fixture projections,
// so every Phase-6 screen can be built + tested against the frozen contracts
// without a live daemon. It returns fixtures THROUGH the same boundary validator
// the real client uses, so the mock can never emit a payload the real boundary
// would reject (dogfoods parse-don't-trust). The real UdsGatewayPort (§6.4) is a
// later per-slice integration gated on daemon Phase 1.5.
import type {
  GatewayPort,
  ProjectionPageParams,
  ProjectionScope,
  SubscribeParams,
} from "./types";
import type {
  Capabilities,
  ProjectionDelta,
  ProjectionPage,
} from "../contracts/index";
import { CONTRACT_VERSION } from "../contracts/index";
import {
  sessionDeltaFixture,
  sessionPageFixture,
} from "../projections/fixtures/proj_session";
import { parseDelta, parseProjectionPage } from "./boundary";

const PROTOCOL_VERSION = 1;

export class MockGatewayPort implements GatewayPort {
  async get_projection(
    name: string,
    _scope?: ProjectionScope,
    _page?: ProjectionPageParams,
  ): Promise<ProjectionPage> {
    if (name === "Session") {
      return parseProjectionPage("Session", sessionPageFixture);
    }
    throw new Error(`MockGatewayPort: no fixture for projection "${name}"`);
  }

  async *subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta> {
    // Symmetric with get_projection: an unrecognized projection fails fast
    // rather than silently streaming nothing (which would mask wiring bugs).
    if (params.projection !== "Session") {
      throw new Error(
        `MockGatewayPort: no fixture for subscribe projection "${params.projection}"`,
      );
    }
    yield parseDelta(sessionDeltaFixture);
  }

  async get_capabilities(): Promise<Capabilities> {
    return {
      protocol_version: PROTOCOL_VERSION,
      contract_version: CONTRACT_VERSION,
    };
  }
}
