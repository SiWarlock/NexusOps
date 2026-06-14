//! Phase 3.3b — Codex auth resolution + execution-profile binding (§15 #8). RED-first.
//! ARCHITECTURE §15 (#8 execution-profile binding — resolved at approval time, recorded in
//! SessionStarted, NO silent account-hopping; the `codex doctor` multi-auth analog → our hard refuse),
//! §5.1 (the frozen ExecutionProfile). Foundation: docs/planning/0.3-codex-schema-research.md §5.
//!
//! NON-cat-1: deterministic auth resolution + profile binding; NO live auth read, NO spawn.

use nexusops_shared::ids::ExecutionProfileId;
use nexusopsd::harness::codex::auth::{
    auth_env_mutations, bind_codex_profile, resolve_codex_auth, AuthSources, CodexAuthError,
    CodexAuthMethod,
};

/// the env-var names removed by a set of mutations.
fn removed(muts: Vec<nexusopsd::terminal::EnvMutation>) -> Vec<String> {
    muts.into_iter()
        .filter(|e| e.value.is_none())
        .map(|e| e.key)
        .collect()
}

// ---- #3 — resolve exactly one auth source → method + the kept env-var (§15 #8) ------------------

#[test]
fn test_resolve_auth_single_method() {
    // spec(§15) — exactly ONE resolvable auth source → Ok(ResolvedAuth{method, source_env}). An env
    // API key alone → ApiKey (source_env = that var); auth.json ChatGPT alone → Chatgpt (no env source);
    // a CODEX_ACCESS_TOKEN alone → ChatgptAuthTokens. The resolved+pinned method (§15 #8).
    let api = resolve_codex_auth(&AuthSources {
        openai_api_key: Some("sk-x".into()),
        ..AuthSources::default()
    })
    .expect("api ok");
    assert_eq!(api.method, CodexAuthMethod::ApiKey);
    assert_eq!(api.source_env.as_deref(), Some("OPENAI_API_KEY"));

    let chatgpt = resolve_codex_auth(&AuthSources {
        auth_json_chatgpt: true,
        ..AuthSources::default()
    })
    .expect("chatgpt ok");
    assert_eq!(chatgpt.method, CodexAuthMethod::Chatgpt);
    assert_eq!(
        chatgpt.source_env, None,
        "auth.json is a file, no env source"
    );

    let token = resolve_codex_auth(&AuthSources {
        codex_access_token: Some("oauth-x".into()),
        ..AuthSources::default()
    })
    .expect("token ok");
    assert_eq!(token.method, CodexAuthMethod::ChatgptAuthTokens);
    assert_eq!(token.source_env.as_deref(), Some("CODEX_ACCESS_TOKEN"));
}

// ---- #4 — refuse AMBIGUOUS auth, fail-closed (§15 #8 no-silent-account-hop) ---------------------

#[test]
fn test_resolve_auth_ambiguous_refused() {
    // spec(§15) — ≥2 distinct auth METHODS (an env API key AND auth.json ChatGPT tokens) →
    // Err(Ambiguous): never a silent pick (the `codex doctor` multi-auth warning → our hard refuse).
    let cross = resolve_codex_auth(&AuthSources {
        openai_api_key: Some("sk-x".into()),
        auth_json_chatgpt: true,
        ..AuthSources::default()
    });
    assert_eq!(cross, Err(CodexAuthError::Ambiguous));

    // no auth at all → also fail-closed (can't launch without a resolved method).
    assert_eq!(
        resolve_codex_auth(&AuthSources::default()),
        Err(CodexAuthError::NoAuth)
    );
}

// ---- #5 — env-hygiene guarantees EXACTLY ONE auth source reaches the child (§15 #8) -------------

#[test]
fn test_auth_env_hygiene_pins_resolved() {
    // spec(§15) — the real §15 #8 pin: after env-hygiene, the child sees EXACTLY ONE auth source — so
    // Codex cannot pick by an internal precedence (a silent account selection). Two API-key vars with
    // the SAME value = one effective credential → Ok + the duplicate stripped; DIFFERENT values →
    // Err(Ambiguous) (we don't own a contestable precedence). The non-chosen methods' vars are stripped.

    // same-value API-key dup → Ok(ApiKey); exactly the resolved var survives (the dup + token stripped).
    let same = resolve_codex_auth(&AuthSources {
        openai_api_key: Some("k".into()),
        codex_api_key: Some("k".into()),
        ..AuthSources::default()
    })
    .expect("same-value dup resolves");
    assert_eq!(same.method, CodexAuthMethod::ApiKey);
    let r = removed(auth_env_mutations(same.source_env.as_deref()));
    // the three auth env vars minus the kept one are stripped → exactly ONE survives.
    let kept: Vec<&str> = ["OPENAI_API_KEY", "CODEX_API_KEY", "CODEX_ACCESS_TOKEN"]
        .into_iter()
        .filter(|v| !r.contains(&v.to_string()))
        .collect();
    assert_eq!(
        kept,
        vec![same.source_env.as_deref().unwrap()],
        "exactly one auth env survives"
    );
    assert!(
        r.contains(&"CODEX_ACCESS_TOKEN".to_string()),
        "the token var is stripped"
    );

    // different-value API-key vars → REFUSE (no contestable precedence).
    assert_eq!(
        resolve_codex_auth(&AuthSources {
            openai_api_key: Some("k1".into()),
            codex_api_key: Some("k2".into()),
            ..AuthSources::default()
        }),
        Err(CodexAuthError::Ambiguous)
    );

    // a resolved ChatGPT (file) method strips ALL auth env vars (only auth.json remains = one source).
    let cg = removed(auth_env_mutations(None));
    for v in ["OPENAI_API_KEY", "CODEX_API_KEY", "CODEX_ACCESS_TOKEN"] {
        assert!(cg.contains(&v.to_string()), "Chatgpt method strips {v}");
    }
}

// ---- #6 — profile binding records the resolved tuple + a valid ExecutionProfileId (§15 #8) ------

#[test]
fn test_profile_binding_records_resolved() {
    // spec(§15/§5.1) — the resolved (auth, model, provider, approval, sandbox, profile) → a
    // CodexExecutionProfile carrying the tuple + a valid ExecutionProfileId handle (the §15 #8
    // record-at-start shape). `sandbox`/`approval_policy` are REQUIRED fields (no Default fallback →
    // 3.3b can't silently ship a containment value; the value choice is 3.3c, Q1 guard).
    let resolved = resolve_codex_auth(&AuthSources {
        auth_json_chatgpt: true,
        ..AuthSources::default()
    })
    .unwrap();
    let p = bind_codex_profile(
        resolved,
        "keychain://codex/sess-x",
        "gpt-5.5",
        "openai",
        "on-request",
        "workspace-write",
        Some("default"),
    );
    assert_eq!(p.auth_method, CodexAuthMethod::Chatgpt);
    assert_eq!(p.model, "gpt-5.5");
    assert_eq!(p.model_provider, "openai");
    assert_eq!(p.approval_policy, "on-request");
    assert_eq!(p.sandbox, "workspace-write");
    assert_eq!(p.profile.as_deref(), Some("default"));
    assert!(ExecutionProfileId::parse(p.execution_profile_id.as_str()).is_ok());
}

// ---- #7 — the profile never leaks the secret, ACROSS METHODS (§15 #4/#8 keychain-ref only) ------

#[test]
fn test_profile_no_secret_leak() {
    // spec(§15) — the bound profile carries a keychain-REF pointer + the env-var NAME, NEVER the secret
    // VALUE; Debug is secret-free FOR EVERY method (an `sk-` API key AND an OAuth-token-shaped secret —
    // the non-`sk-` case must hold too). The secret value never flows resolve→bind→profile (resolution
    // keeps only the source-env NAME); the type structurally holds no raw secret.
    const API_SECRET: &str = "sk-LEAK-CANARY-APIKEY-9999";
    const OAUTH_SECRET: &str = "oauth-LEAK-CANARY-TOKEN-9999";

    let api = resolve_codex_auth(&AuthSources {
        openai_api_key: Some(API_SECRET.into()),
        ..AuthSources::default()
    })
    .unwrap();
    let p_api = bind_codex_profile(
        api,
        "keychain://codex/apikey-ref",
        "gpt-5.5",
        "openai",
        "on-request",
        "workspace-write",
        None,
    );
    let dbg_api = format!("{p_api:?}");
    assert!(
        !dbg_api.contains(API_SECRET),
        "API-key secret never in the profile Debug"
    );
    assert!(
        dbg_api.contains("keychain://codex/apikey-ref"),
        "carries the keychain-ref pointer"
    );

    let tok = resolve_codex_auth(&AuthSources {
        codex_access_token: Some(OAUTH_SECRET.into()),
        ..AuthSources::default()
    })
    .unwrap();
    let p_tok = bind_codex_profile(
        tok,
        "keychain://codex/token-ref",
        "gpt-5.5",
        "openai",
        "on-request",
        "workspace-write",
        None,
    );
    assert!(
        !format!("{p_tok:?}").contains(OAUTH_SECRET),
        "OAuth-token secret never in the profile Debug (the non-sk- case)"
    );
}
