//! Codex auth resolution + execution-profile binding (§15 #8 — no silent account-hopping).
//!
//! `resolve_codex_auth` resolves EXACTLY ONE auth source or fails closed (the `codex doctor`
//! multi-auth warning → our hard refuse): ≥2 distinct methods → `Ambiguous`; two API-key env vars with
//! CONFLICTING values → `Ambiguous` (we never own a contestable precedence); a same-value duplicate is
//! one effective credential (canonical-pinned, the dup stripped by env-hygiene). The resolved method +
//! the kept env-var NAME bind into a daemon-internal [`CodexExecutionProfile`] (+ the frozen
//! `ExecutionProfileId` handle the §15 #8 binding records at `SessionStarted`); the auth field is a
//! **keychain-ref pointer, NEVER the secret** (§15 #4). The secret VALUE is compared in `resolve` then
//! DROPPED — only the var NAME flows onward, so the profile (and its `Debug`) is secret-free.

use nexusops_shared::ids::ExecutionProfileId;

use crate::terminal::EnvMutation;

/// The resolved Codex auth method (the research §5 `CodexAuth` enum). A tag only — no secret.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CodexAuthMethod {
    /// an API key (`OPENAI_API_KEY` / `CODEX_API_KEY`)
    ApiKey,
    /// browser-OAuth ChatGPT, persisted in `~/.codex/auth.json`
    Chatgpt,
    /// external/ephemeral access tokens (`CODEX_ACCESS_TOKEN` — the headless-OAuth path)
    ChatgptAuthTokens,
    /// an agent-identity access token (resolution doesn't detect it from env — HITL Open-Q #8)
    AgentIdentity,
}

/// The auth sources present in the resolution environment — the env-var VALUES (so resolution can
/// compare same-vs-different) + the `auth.json` ChatGPT-token presence. **No `Debug` derive** — this
/// transiently holds secret env values; it must never be logged (the secret is compared then dropped).
#[derive(Default)]
pub struct AuthSources {
    pub openai_api_key: Option<String>,
    pub codex_api_key: Option<String>,
    pub codex_access_token: Option<String>,
    pub auth_json_chatgpt: bool,
}

/// The resolved auth: the method + the env-var NAME carrying the credential (`None` for the file-based
/// `auth.json` ChatGPT). The NAME (not the value) is safe to store + `Debug` — env-hygiene strips every
/// other auth env var so exactly this one source reaches the child.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ResolvedAuth {
    pub method: CodexAuthMethod,
    pub source_env: Option<String>,
}

/// Why auth resolution refused to pin a single method (fail-closed — no launch).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CodexAuthError {
    /// ≥2 distinct methods, or two API-key vars with conflicting values (no contestable precedence).
    Ambiguous,
    /// no resolvable auth source at all.
    NoAuth,
}

/// The auth-bearing env vars, in canonical preference order (the API-key pick prefers `OPENAI_API_KEY`).
const AUTH_ENV: [&str; 3] = ["OPENAI_API_KEY", "CODEX_API_KEY", "CODEX_ACCESS_TOKEN"];

/// Resolve EXACTLY ONE auth source (§15 #8) or fail closed. Two API-key vars with the SAME value =
/// one effective credential (canonical-pinned to `OPENAI_API_KEY`); DIFFERENT values → `Ambiguous`.
/// ≥2 distinct methods → `Ambiguous`. None → `NoAuth`. The secret value is compared here then dropped —
/// the result carries only the method + the kept env-var NAME.
pub fn resolve_codex_auth(sources: &AuthSources) -> Result<ResolvedAuth, CodexAuthError> {
    // the API-key method (OPENAI_API_KEY / CODEX_API_KEY), canonical pick = OPENAI first.
    let api_vars: Vec<(&str, &str)> = [
        ("OPENAI_API_KEY", sources.openai_api_key.as_deref()),
        ("CODEX_API_KEY", sources.codex_api_key.as_deref()),
    ]
    .into_iter()
    .filter_map(|(name, val)| val.map(|v| (name, v)))
    .collect();
    let api_method = match api_vars.as_slice() {
        [] => None,
        [(name, _)] => Some(ResolvedAuth {
            method: CodexAuthMethod::ApiKey,
            source_env: Some((*name).to_string()),
        }),
        vars => {
            // ≥2 API-key vars: same value = one credential (pin the canonical first); different = refuse.
            let first = vars[0].1;
            if vars.iter().all(|(_, v)| *v == first) {
                Some(ResolvedAuth {
                    method: CodexAuthMethod::ApiKey,
                    source_env: Some(vars[0].0.to_string()),
                })
            } else {
                return Err(CodexAuthError::Ambiguous);
            }
        }
    };

    let mut methods: Vec<ResolvedAuth> = Vec::new();
    if let Some(api) = api_method {
        methods.push(api);
    }
    if sources.codex_access_token.is_some() {
        methods.push(ResolvedAuth {
            method: CodexAuthMethod::ChatgptAuthTokens,
            source_env: Some("CODEX_ACCESS_TOKEN".to_string()),
        });
    }
    if sources.auth_json_chatgpt {
        methods.push(ResolvedAuth {
            method: CodexAuthMethod::Chatgpt,
            source_env: None,
        });
    }

    if methods.len() >= 2 {
        return Err(CodexAuthError::Ambiguous); // ≥2 distinct methods → no silent pick
    }
    // exactly 0 or 1 remains; take the single resolved method (the canonical-first), else NoAuth.
    methods.into_iter().next().ok_or(CodexAuthError::NoAuth)
}

/// The env-hygiene mutations that pin the resolved auth source (§15 #8): strip EVERY auth env var
/// except `source_env` (the kept credential) so exactly one auth source reaches the child — no
/// account-hop off a non-chosen var, no internal-precedence pick among duplicates. `source_env=None`
/// (the file-based ChatGPT method) strips all three, leaving only `auth.json`. The `NEXUSOPS_SESSION_ID`
/// correlation key is added by the caller ([`CodexLaunchSpec::env_mutations`](super::launch::CodexLaunchSpec::env_mutations)).
pub fn auth_env_mutations(source_env: Option<&str>) -> Vec<EnvMutation> {
    AUTH_ENV
        .iter()
        .filter(|var| Some(**var) != source_env)
        .map(|var| EnvMutation::remove(*var))
        .collect()
}

/// The daemon-internal Codex execution profile (§15 #8 binding) — the resolved tuple + the frozen
/// `ExecutionProfileId` handle recorded at `SessionStarted`. `auth_keychain_ref` is a NON-secret
/// pointer; the struct holds NO raw secret, so `Debug`/serialization is secret-free (§15 #4). **No
/// `Default` derive** — `sandbox`/`approval_policy` are REQUIRED (3.3b can't silently ship a containment
/// value; that choice is 3.3c, the Q1 guard).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CodexExecutionProfile {
    pub auth_method: CodexAuthMethod,
    pub auth_source_env: Option<String>,
    pub auth_keychain_ref: String,
    pub model: String,
    pub model_provider: String,
    pub approval_policy: String,
    pub sandbox: String,
    pub profile: Option<String>,
    pub execution_profile_id: ExecutionProfileId,
}

/// Bind the resolved auth + the launch config into a [`CodexExecutionProfile`] (§15 #8), minting the
/// `ExecutionProfileId` handle. `keychain_ref` is a non-secret pointer (never the secret). `sandbox`
/// + `approval_policy` are REQUIRED (the Q1 guard — no default containment value).
#[allow(clippy::too_many_arguments)] // the §15 #8 resolved tuple — a faithful binding, not over-args
pub fn bind_codex_profile(
    resolved: ResolvedAuth,
    keychain_ref: &str,
    model: &str,
    model_provider: &str,
    approval_policy: &str,
    sandbox: &str,
    profile: Option<&str>,
) -> CodexExecutionProfile {
    CodexExecutionProfile {
        auth_method: resolved.method,
        auth_source_env: resolved.source_env,
        auth_keychain_ref: keychain_ref.to_string(),
        model: model.to_string(),
        model_provider: model_provider.to_string(),
        approval_policy: approval_policy.to_string(),
        sandbox: sandbox.to_string(),
        profile: profile.map(|s| s.to_string()),
        execution_profile_id: ExecutionProfileId::new(),
    }
}
