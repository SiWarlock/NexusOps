//! Desktop-addendum objects (ARCHITECTURE §5.3 / DATA_MODEL §6).
//!
//! Closes the SOM gap (DFR §7). MVP-live: **LocalRunner** (daemon execution
//! surface; sessions bind to it) + **EventProjection** (projection catalog).
//! Dormant iOS scaffolding `[DEFERRED]`: **Device**, **RemoteClient**. The freeze
//! pins each object's **type + identity (ID prefix)**, not its behavior.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The 4 desktop-addendum object kinds (§5.3). Serialized snake_case.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DesktopObjectKind {
    // LocalRunner — daemon execution surface (MVP-live)
    // EventProjection — projection catalog/metadata (MVP-live)
    // Device — registered machine; trust-boundary anchor (deferred, iOS)
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

    /// `true` for the dormant iOS-scaffolding objects (Device, RemoteClient);
    /// `false` for the MVP-live ones. Freezing identity, not behavior (§5.3).
    pub fn is_deferred(self) -> bool {
        matches!(self, Self::Device | Self::RemoteClient)
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
