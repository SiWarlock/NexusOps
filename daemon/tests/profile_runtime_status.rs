//! P5.3b/085 — piece 3: the §7.2 runtime-status RE-DERIVATION. The pure `derive_runtime_status` classifier
//! (LESSON §36/§41 pure-classifier family) — the RUNTIME status is recomputed from the live inputs in strict
//! precedence, NOT trusted from the persisted config row. Recomputed-not-replayed: a live-input flip on a
//! FIXED config status changes the derived status (the LESSON §41 discipline).

use nexusops_shared::status::ExecutionProfile as EP;
use nexusopsd::profiles::secret::{derive_runtime_status, RuntimeInputs};

/// inputs with one live signal flipped on (the rest default-false).
fn inputs(
    self_test: Option<EP>,
    in_use: bool,
    rate_limited: bool,
    auth_expired: bool,
) -> RuntimeInputs {
    RuntimeInputs {
        self_test,
        in_use,
        rate_limited,
        auth_expired,
    }
}

#[test]
fn runtime_status_precedence_strict() {
    // spec(§7.2 / §5.1) — strict precedence (worst-first): a terminal config (Disabled) is honored over ANY
    // live signal; else misconfigured > auth_expired > rate_limited > in_use > the config status.
    // Disabled config is honored regardless of live inputs.
    assert_eq!(
        derive_runtime_status(
            EP::Disabled,
            &inputs(Some(EP::Misconfigured), true, true, true)
        ),
        EP::Disabled,
        "a terminal Disabled config is honored over every live signal"
    );
    // misconfigured beats every other live signal.
    assert_eq!(
        derive_runtime_status(
            EP::Available,
            &inputs(Some(EP::Misconfigured), true, true, true)
        ),
        EP::Misconfigured
    );
    // auth_expired beats rate_limited + in_use.
    assert_eq!(
        derive_runtime_status(EP::Available, &inputs(None, true, true, true)),
        EP::AuthExpired
    );
    // rate_limited beats in_use.
    assert_eq!(
        derive_runtime_status(EP::Available, &inputs(None, true, true, false)),
        EP::RateLimited
    );
    // in_use beats the bare config.
    assert_eq!(
        derive_runtime_status(EP::Available, &inputs(None, true, false, false)),
        EP::InUse
    );
}

#[test]
fn runtime_status_total_else_falls_back_to_config() {
    // spec(§7.2) — total else: NO live condition → the configured/intended status (recompute returns the
    // config when nothing live overrides it). Covers a couple of config values to pin the passthrough.
    assert_eq!(
        derive_runtime_status(EP::Available, &inputs(None, false, false, false)),
        EP::Available
    );
    assert_eq!(
        derive_runtime_status(EP::Active, &inputs(None, false, false, false)),
        EP::Active
    );
    // a healthy self-test (None) does not override the config either.
    assert_eq!(
        derive_runtime_status(EP::Available, &inputs(None, false, false, false)),
        EP::Available
    );
}

#[test]
fn runtime_status_recomputed_not_replayed() {
    // spec(LESSON §41 — live-read-recompute) — on a FIXED config status (Available), flipping a live input
    // changes the derived RUNTIME status → it is recomputed from live inputs each read, NOT replayed from the
    // persisted row (the config status is unchanged across both calls; only the live input differs).
    let config = EP::Available;
    let idle = derive_runtime_status(config, &inputs(None, false, false, false));
    let busy = derive_runtime_status(config, &inputs(None, true, false, false));
    assert_eq!(idle, EP::Available);
    assert_eq!(busy, EP::InUse);
    assert_ne!(
        idle, busy,
        "the SAME config row yields different runtime status as live inputs flip"
    );
}
