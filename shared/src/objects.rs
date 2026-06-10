//! Desktop-addendum objects (ARCHITECTURE §5.3 / DATA_MODEL §6).
//!
//! Closes the SOM gap (DFR §7). MVP-live: **LocalRunner** (daemon execution
//! surface; sessions bind to it) + **EventProjection** (projection catalog) + the local
//! **Device** (the stable desktop-host identity, registered at cold-start per §16 — user-ruled
//! Option A). `[DEFERRED]`: **RemoteClient** + the iOS multi-device/pairing dimension of Device.
//! The freeze pins each object's **type + identity (ID prefix)**, not its behavior.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The 4 desktop-addendum object kinds (§5.3). Serialized snake_case.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DesktopObjectKind {
    // LocalRunner — daemon execution surface (MVP-live)
    // EventProjection — projection catalog/metadata (MVP-live)
    // Device — the desktop host (MVP-live: stable per-host identity, registered at
    //   cold-start §16); the iOS multi-device/pairing dimension stays deferred
    // RemoteClient — authenticated remote connection (deferred, iOS)
    // (plain comments, not `///`: keep schemars emitting a flat string `enum`,
    // not a oneOf-of-const, so Zod/Pydantic generate uniform string enums.)
    LocalRunner,
    EventProjection,
    Device,
    RemoteClient,
}

impl DesktopObjectKind {
    /// All 4 kinds, declaration order.
    pub const ALL: &'static [Self] = &[
        Self::LocalRunner,
        Self::EventProjection,
        Self::Device,
        Self::RemoteClient,
    ];

    /// `true` for the deferred surface — **RemoteClient** + the iOS multi-device/pairing
    /// dimension of Device; `false` for the MVP-live kinds (LocalRunner, EventProjection,
    /// and the **local desktop-host Device** — registered at cold-start §16, user-ruled
    /// Option A). Freezing identity, not behavior (§5.3).
    pub fn is_deferred(self) -> bool {
        matches!(self, Self::RemoteClient)
    }

    /// The object's identity prefix (DATA_MODEL §6; `prj_`→`eprj_` de-collided at
    /// freeze to avoid the `proj_` near-homograph).
    pub fn id_prefix(self) -> &'static str {
        match self {
            Self::LocalRunner => "lr_",
            Self::EventProjection => "eprj_",
            Self::Device => "dev_",
            Self::RemoteClient => "rc_",
        }
    }
}

/// A desktop-addendum minted `<prefix><ULID>` newtype (§5.3). A sibling of `ids::minted_id!`,
/// but keyed off [`DesktopObjectKind`] (its `id_prefix()`) so the **frozen-22 `IdKind` set
/// stays untouched** — the desktop ids are NOT new `IdKind` variants. Reuses the shared
/// [`crate::ids::IdError`] + ULID format (Crockford base32, fail-closed parse, §15).
macro_rules! desktop_minted_id {
    ($(#[$meta:meta])* $name:ident, $kind:expr, $prefix:literal) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// the [`DesktopObjectKind`] this newtype represents
            pub const KIND: DesktopObjectKind = $kind;
            /// the ULID prefix this newtype carries (== `KIND.id_prefix()`)
            pub const PREFIX: &'static str = $prefix;

            /// Mint a fresh `<prefix><ULID>`.
            pub fn new() -> Self {
                Self(format!("{}{}", $prefix, ulid::Ulid::new()))
            }

            /// The full string form.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Parse + validate: correct prefix and a well-formed ULID body (fail-closed, §15).
            pub fn parse(s: &str) -> Result<Self, crate::ids::IdError> {
                let body = s
                    .strip_prefix($prefix)
                    .ok_or(crate::ids::IdError::WrongPrefix)?;
                ulid::Ulid::from_string(body).map_err(|_| crate::ids::IdError::BadUlid)?;
                Ok(Self(s.to_string()))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

desktop_minted_id!(
    /// `dev_<ULID>` — the registered desktop host (§5.3 Device; trust-boundary anchor).
    /// Register-if-absent: minted once, reused across restarts.
    DeviceId,
    DesktopObjectKind::Device,
    "dev_"
);
desktop_minted_id!(
    /// `lr_<ULID>` — a daemon execution surface (§5.3 LocalRunner), minted per daemon start.
    LocalRunnerId,
    DesktopObjectKind::LocalRunner,
    "lr_"
);
