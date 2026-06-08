//! Resource/lease arbitration for the persistence core (ADR-008 / §7.2 / §17).
//!
//! Two mechanisms:
//! - [`lease`] — cross-restart SQLite leases + a **monotonic fencing token** (safety rule #6):
//!   the load-bearing primitive the Phase-2 Action Gateway uses to reject a stale-token
//!   mutation (`fencing_conflict`). The free functions write through the daemon's single
//!   writable `Connection` (driven by `EventStore`'s delegating methods, the `drain_once`
//!   precedent) — never a second writer (Forbidden #3 / LESSON §3).
//! - [`pidlock`] — single-instance enforcement via a std advisory file lock (no DB): the OS
//!   holds the lock against the open fd → auto-released on process death, immune to PID reuse.
//!   The cold-start call site is the 1.6 bootstrap (§16 ordering).
//!
//! **Daemon-internal** — `leases` is not a `shared/` contract (the UI/Brain never read it),
//! so no CONTRACT_VERSION bump (analogous to the 1.3 outbox).

mod lease;
mod pidlock;

// The lease types are public (they cross `EventStore`'s public method signatures); the free
// functions are crate-internal — reached only through the connection-owning `EventStore`.
pub(crate) use lease::{acquire, release, renew, validate_held};
pub use lease::{FencingToken, Lease, LeaseError, LeaseKind, OwnerId, ResourceId};
// pidlock is a standalone OS-file-lock guard (no DB) — the 1.6 bootstrap is the call site.
pub use pidlock::{PidLock, PidLockError};
