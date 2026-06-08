//! Cross-restart resource leases + monotonic fencing (ADR-008 / §7.2 / §17 / safety rule #6).
//!
//! A lease is one row per `(resource_id, lease_kind)` in the `leases` table (migration 5),
//! carrying a **monotonic fencing high-water mark** that only ever increments and survives
//! restart (it is persisted in the row — never an in-memory counter, never a `MAX()` over
//! deletable rows). `acquire` mints `token+1` (1 for a brand-new row) when the slot is free
//! or expired, and refuses a slot held by a *live* lease of a different owner. `renew` is
//! owner-and-token-and-expiry guarded (renew ≠ reclaim; the token is unchanged). `release`
//! frees the slot but **preserves** the high-water mark so the next acquire still increments.
//!
//! **Authority oracle ([`validate_held`], human-ruled Option B):** a token is authoritative
//! iff the caller is the current owner AND its token equals the high-water mark AND the lease
//! is still live (`expires_at > now`). Authority is tied to a **live** lease, not a merely-
//! unsuperseded token — an EXPIRED holder is rejected even if no one has reclaimed yet (the
//! gap §17 line 387 left open). This is exactly the check the Phase-2 Action Gateway calls to
//! reject a stale-token mutation (safety rule #6 → `fencing_conflict`, hard-conflict card,
//! never auto-resolved).
//!
//! All writes go through the **single writable `Connection`** the daemon owns (Forbidden #3 /
//! LESSON §3 — never a second writer); the read-modify-write paths use `BEGIN IMMEDIATE` so
//! the high-water read + the mint commit atomically (LESSON §3), matching the `append` writer.

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use crate::clock::Clock;

/// Opaque resource key the lease arbitrates (the caller's worktree/repo/session id). The
/// lease layer treats it as an opaque string — the Phase-2 gateway supplies the concrete id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceId(pub String);

/// The kind of lease over a resource. A `TEXT` wire value (not yet a closed enum — the
/// Phase-2 gateway consumer dictates the kinds); 1.4 seeds the single MVP kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseKind(pub String);

impl LeaseKind {
    /// the seeded MVP lease kind (mutation arbitration). The closed-enum freeze defers to
    /// the Phase-2 gateway that actually dictates kinds.
    pub const RESOURCE_MUTATION: &'static str = "resource_mutation";

    pub fn resource_mutation() -> Self {
        Self(Self::RESOURCE_MUTATION.to_string())
    }
}

/// The lease holder. A minimal typed wrapper over `TEXT` — NOT yet bound to one of the 22
/// frozen `shared/` IDs (the gateway binds the concrete owner kind in Phase 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerId(pub String);

/// A monotonic fencing token — load-bearing safety invariant (safety rule #6), so a newtype
/// rather than a bare int. Stored as a SQLite `INTEGER` (i64) at the boundary; the domain
/// type is `u64` (a token is a non-negative counter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FencingToken(pub u64);

impl FencingToken {
    /// SQLite stores `INTEGER` as i64 — convert, failing closed on the (practically
    /// impossible) overflow rather than silently wrapping a load-bearing token.
    fn to_sqlite(self) -> Result<i64, LeaseError> {
        i64::try_from(self.0).map_err(|_| LeaseError::TokenOverflow)
    }
    fn from_sqlite(v: i64) -> Result<Self, LeaseError> {
        u64::try_from(v)
            .map(FencingToken)
            .map_err(|_| LeaseError::TokenOverflow)
    }
}

/// A granted lease handle — what `acquire`/`renew` return. The holder presents this (its
/// `fencing_token` + `owner_id`) to `renew`/`release`/`validate_held`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease {
    pub resource_id: ResourceId,
    pub lease_kind: LeaseKind,
    pub owner_id: OwnerId,
    pub fencing_token: FencingToken,
    pub acquired_at: String,
    pub heartbeat_at: String,
    pub expires_at: String,
}

/// Typed lease failures — fail-closed (§15/§17): a guarded op that can't prove the caller is
/// the live holder returns an error, never a silent success.
#[derive(Debug, thiserror::Error)]
pub enum LeaseError {
    /// a live lease is held by a different owner — acquire refused, nothing mutated.
    #[error("lease held by another owner")]
    Held { owner_id: String },
    /// renew/release by a non-holder (wrong owner / stale token / expired) — state unchanged.
    #[error("not the current lease holder (stale token, wrong owner, or expired)")]
    NotHolder,
    /// a fencing token outside the i64 range SQLite can store (defensive; unreachable in MVP).
    #[error("fencing token out of range")]
    TokenOverflow,
    #[error("lease db error: {0}")]
    Db(rusqlite::Error),
}

/// the current persisted holder row (owner, high-water token, expires_at) for a slot.
type SlotRow = (Option<String>, i64, Option<String>);

fn read_slot(
    conn: &Connection,
    resource_id: &ResourceId,
    lease_kind: &LeaseKind,
) -> Result<Option<SlotRow>, LeaseError> {
    conn.query_row(
        "SELECT owner_id, fencing_token, expires_at FROM leases \
         WHERE resource_id = ?1 AND lease_kind = ?2",
        params![resource_id.0, lease_kind.0],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .optional()
    .map_err(LeaseError::Db)
}

/// Acquire the lease when the slot is free OR expired; mint a fencing token **strictly
/// greater than any prior token ever issued** for this `(resource_id, lease_kind)` (the
/// persisted high-water mark + 1; 1 for a brand-new row). Refuse — with a typed `Held`
/// error and zero mutation — when a *live* lease is held by a different owner (§7.2).
/// Read-modify-write under `BEGIN IMMEDIATE` so the high-water read + mint are atomic.
pub(crate) fn acquire(
    conn: &mut Connection,
    resource_id: &ResourceId,
    lease_kind: &LeaseKind,
    owner_id: &OwnerId,
    ttl_secs: i64,
    clock: &dyn Clock,
) -> Result<Lease, LeaseError> {
    let now = clock.now_rfc3339();
    let expires_at = clock.now_plus_secs(ttl_secs);

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(LeaseError::Db)?;

    let new_token: i64 = match read_slot(&tx, resource_id, lease_kind)? {
        None => 1, // brand-new slot
        Some((owner, token, expires)) => {
            // live = a holder is recorded AND the lease has not expired (lexical Z-UTC
            // compare, LESSON §5). A live lease held by another owner is refused. The
            // holder triple (owner/acquired/expires) is always set or fully-NULLed
            // together by acquire/renew/release, so `owner.is_some()` with `expires=NULL`
            // never occurs; `is_some_and` treating it as not-live is the safe default.
            let live = owner.is_some() && expires.as_deref().is_some_and(|e| e > now.as_str());
            if live {
                // tx drops → rollback: the refusal mutates nothing.
                return Err(LeaseError::Held {
                    owner_id: owner.unwrap_or_default(),
                });
            }
            token + 1 // reclaim a free/expired slot: strictly-greater token (no reuse)
        }
    };

    tx.execute(
        "INSERT INTO leases \
           (resource_id, lease_kind, owner_id, fencing_token, acquired_at, heartbeat_at, expires_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6) \
         ON CONFLICT(resource_id, lease_kind) DO UPDATE SET \
           owner_id = excluded.owner_id, \
           fencing_token = excluded.fencing_token, \
           acquired_at = excluded.acquired_at, \
           heartbeat_at = excluded.heartbeat_at, \
           expires_at = excluded.expires_at",
        params![resource_id.0, lease_kind.0, owner_id.0, new_token, now, expires_at],
    )
    .map_err(LeaseError::Db)?;
    tx.commit().map_err(LeaseError::Db)?;

    Ok(Lease {
        resource_id: resource_id.clone(),
        lease_kind: lease_kind.clone(),
        owner_id: owner_id.clone(),
        fencing_token: FencingToken::from_sqlite(new_token)?,
        acquired_at: now.clone(),
        heartbeat_at: now,
        expires_at,
    })
}

/// Renew an owned lease (renew ≠ reclaim): extend `expires_at` + bump `heartbeat_at`, with
/// the **fencing token unchanged**. Owner-AND-token-AND-expiry guarded — a stale/wrong-token/
/// expired renew matches no row → `NotHolder`, state unchanged.
pub(crate) fn renew(
    conn: &Connection,
    lease: &Lease,
    ttl_secs: i64,
    clock: &dyn Clock,
) -> Result<Lease, LeaseError> {
    let now = clock.now_rfc3339();
    let new_expires = clock.now_plus_secs(ttl_secs);
    let token = lease.fencing_token.to_sqlite()?;

    // the expiry guard (`expires_at > ?7`) binds `now` in its OWN slot — not reusing
    // the `heartbeat_at` slot — so the liveness check can't silently track the wrong
    // value if the param order ever changes (the renew rejects an already-expired lease).
    let n = conn
        .execute(
            "UPDATE leases SET expires_at = ?1, heartbeat_at = ?2 \
             WHERE resource_id = ?3 AND lease_kind = ?4 AND owner_id = ?5 \
               AND fencing_token = ?6 AND expires_at > ?7",
            params![
                new_expires,
                now,
                lease.resource_id.0,
                lease.lease_kind.0,
                lease.owner_id.0,
                token,
                now,
            ],
        )
        .map_err(LeaseError::Db)?;
    if n == 0 {
        return Err(LeaseError::NotHolder);
    }
    Ok(Lease {
        expires_at: new_expires,
        heartbeat_at: now,
        ..lease.clone()
    })
}

/// Release an owned lease: NULL the holder fields (free the slot) but **preserve** the
/// fencing high-water mark so the next acquire still increments. Owner-and-token guarded.
pub(crate) fn release(conn: &Connection, lease: &Lease) -> Result<(), LeaseError> {
    let token = lease.fencing_token.to_sqlite()?;
    let n = conn
        .execute(
            "UPDATE leases SET owner_id = NULL, acquired_at = NULL, heartbeat_at = NULL, \
               expires_at = NULL \
             WHERE resource_id = ?1 AND lease_kind = ?2 AND owner_id = ?3 AND fencing_token = ?4",
            params![
                lease.resource_id.0,
                lease.lease_kind.0,
                lease.owner_id.0,
                token
            ],
        )
        .map_err(LeaseError::Db)?;
    if n == 0 {
        return Err(LeaseError::NotHolder);
    }
    Ok(())
}

/// The authority oracle (safety rule #6 / §17, human-ruled Option B). `true` iff the caller
/// is the current owner AND its token equals the high-water mark AND the lease is still live
/// (`expires_at > now`). A free slot, an expired lease, a superseded token, or a wrong owner
/// all yield `false`. This is THE check the Phase-2 gateway calls before a mutation — a
/// `false` here is the §17 `fencing_conflict` path.
pub(crate) fn validate_held(
    conn: &Connection,
    resource_id: &ResourceId,
    lease_kind: &LeaseKind,
    owner_id: &OwnerId,
    token: FencingToken,
    now: &str,
) -> Result<bool, LeaseError> {
    let token = token.to_sqlite()?;
    Ok(match read_slot(conn, resource_id, lease_kind)? {
        Some((Some(row_owner), row_token, Some(expires_at))) => {
            row_owner == owner_id.0 && row_token == token && expires_at.as_str() > now
        }
        // no row, or a free slot (owner/expires NULL) → no live authority.
        _ => false,
    })
}

/// Reaper unit (§12): free every lease past `expires_at` (NULL the holder fields, **keep**
/// the fencing high-water mark) and return the reclaimed `(resource_id, lease_kind)` set.
/// Non-expired leases are untouched. Expiry is otherwise lazy (acquire reclaims an expired
/// slot on demand); this is the periodic sweep. The Tokio interval **spawn** is the 1.6
/// bootstrap's job (joins the outbox drainer spawn, §12); 1.4 ships the deterministic unit.
/// Select-then-free under `BEGIN IMMEDIATE` so the scan + frees commit atomically.
///
/// Takes `&mut Connection` (unlike the outbox `drain_once`'s autocommit `&Connection`)
/// because the explicit `transaction_with_behavior(Immediate)` requires `&mut` — same as
/// `acquire`'s read-modify-write.
pub(crate) fn reap_once(
    conn: &mut Connection,
    clock: &dyn Clock,
) -> Result<Vec<(ResourceId, LeaseKind)>, LeaseError> {
    let now = clock.now_rfc3339();
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(LeaseError::Db)?;

    // expired = a holder is recorded AND `expires_at <= now` (the complement of `live`'s
    // `expires_at > now`, so a lease at exactly `now` is reaped). Lexical Z-UTC compare.
    let expired: Vec<(String, String)> = {
        let mut stmt = tx
            .prepare(
                "SELECT resource_id, lease_kind FROM leases \
                 WHERE owner_id IS NOT NULL AND expires_at IS NOT NULL AND expires_at <= ?1",
            )
            .map_err(LeaseError::Db)?;
        let rows = stmt
            .query_map(params![now], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(LeaseError::Db)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(LeaseError::Db)?);
        }
        out
    };

    for (resource_id, lease_kind) in &expired {
        // free the slot but KEEP fencing_token (the next acquire still increments → no reuse).
        tx.execute(
            "UPDATE leases SET owner_id = NULL, acquired_at = NULL, heartbeat_at = NULL, \
               expires_at = NULL \
             WHERE resource_id = ?1 AND lease_kind = ?2",
            params![resource_id, lease_kind],
        )
        .map_err(LeaseError::Db)?;
    }
    tx.commit().map_err(LeaseError::Db)?;

    Ok(expired
        .into_iter()
        .map(|(r, k)| (ResourceId(r), LeaseKind(k)))
        .collect())
}
