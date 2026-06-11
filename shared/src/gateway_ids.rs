//! Gateway-addendum platform IDs (ARCHITECTURE §5.2 / §6.2; RULED Option A, 2026-06-11).
//!
//! `ApprovalId`(`appr_`) + `ActionPlanId`(`aplan_`) are the platform-minted prefixed-ULID
//! identities for the §6.2 Gateway objects. Exactly like the §5.3 desktop objects
//! ([`crate::objects`]), they are **non-cross-product** — keyed off [`GatewayObjectKind`]
//! (a sibling of `DesktopObjectKind`), NOT new `IdKind` variants — so the frozen 22-member
//! `IdKind` cross-product set (§5.2) stays untouched. `IdKind`'s boundary is the PBI
//! cross-product 22 specifically, not "everything crossing daemon→UI". Reuses the shared ULID
//! format + [`crate::ids::IdError`] (Crockford base32, fail-closed parse, §15).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The 2 Gateway-addendum object kinds (§6.2). Serialized snake_case.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GatewayObjectKind {
    // Approval — the human/policy decision object (R-5, §5.1/§6.2)
    // ActionPlan — a bundled multi-step plan (O-3, §6.2)
    // (plain comments, not `///`: keep schemars emitting a flat string `enum`, so Zod/Pydantic
    // generate uniform string enums — mirrors objects.rs.)
    Approval,
    ActionPlan,
}

impl GatewayObjectKind {
    /// Both kinds, declaration order.
    pub const ALL: &'static [Self] = &[Self::Approval, Self::ActionPlan];

    /// The object's identity prefix (§5.2 / §6.2).
    pub fn id_prefix(self) -> &'static str {
        match self {
            Self::Approval => "appr_",
            Self::ActionPlan => "aplan_",
        }
    }
}

/// A Gateway-addendum minted `<prefix><ULID>` newtype (§6.2). A sibling of
/// [`crate::objects`]'s `desktop_minted_id!`, keyed off [`GatewayObjectKind`] so the
/// **frozen-22 `IdKind` set stays untouched** (RULED Option A). Reuses the shared
/// [`crate::ids::IdError`] + ULID format (Crockford base32, fail-closed parse, §15).
macro_rules! gateway_minted_id {
    ($(#[$meta:meta])* $name:ident, $kind:expr, $prefix:literal) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// the [`GatewayObjectKind`] this newtype represents
            pub const KIND: GatewayObjectKind = $kind;
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

gateway_minted_id!(
    /// `appr_<ULID>` — an [`crate::actions::Approval`] (§6.2; the human/policy decision object).
    ApprovalId,
    GatewayObjectKind::Approval,
    "appr_"
);
gateway_minted_id!(
    /// `aplan_<ULID>` — an [`crate::actions::ActionPlan`] (§6.2; a bundled multi-step plan, O-3).
    ActionPlanId,
    GatewayObjectKind::ActionPlan,
    "aplan_"
);
