import { ulid } from "ulid";

// Client-side prefixed-ULID minters for generic `submit_action` intents.
//
// The daemon trusts the wire id verbatim as the `action_requests` TEXT PRIMARY KEY — only the
// dedicated `session.create` method mints daemon-side, so every generic `submit_action` intent
// MUST carry a client-minted id (the daemon `smoke_request` dev-driver precedent). Daemon #090
// adds `ActionRequestId::parse()` (= strip the `<prefix>_` then `ulid::Ulid::from_string`,
// shared/src/ids.rs), so the body MUST be a canonical 26-char uppercase Crockford ULID — which
// `ulid()` emits. (A hyphenated `crypto.randomUUID()` would NOT parse — wrong shape.)

export function mintActionRequestId(): string {
  return `act_${ulid()}`;
}

export function mintProjectId(): string {
  return `proj_${ulid()}`;
}
