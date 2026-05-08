// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.3 — First-run wizard state.
//
// The actual UI lives in the React app under
// `ui/frontend/components/wizard/`. This module exposes the Tauri
// commands that wizard pages call:
//
//   wizard_should_show()    -> bool
//   wizard_state()          -> WizardState
//   wizard_set_privacy(...) -> ()
//   wizard_complete()       -> ()
//
// "Should show" returns true when *either*:
//   - `~/.hypermesh/identity.falcon` does not exist  (no identity yet), or
//   - `~/.hypermesh/wizard.json` does not exist or its `completed` flag
//     is false (user hasn't finished the wizard).
//
// On `wizard_complete()` we write `~/.hypermesh/wizard.json` so the
// shell skips the wizard on subsequent launches.
//
// Identity generation itself is delegated to the daemon via the
// existing `auth.create_session` IPC handler — the wizard page calls
// that through the React Tauri bridge and we only persist user choices
// here.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WizardState {
    /// True until the wizard has been completed once.
    pub should_show: bool,
    /// Identity key file path (existence check).
    pub identity_path: String,
    /// Persisted state file path.
    pub state_path: String,
    /// User-selected privacy mode (Anonymous / Private / Public). Defaults
    /// to "private" until the user actually chooses on Page 2.
    #[serde(default = "default_privacy")]
    pub privacy_mode: String,
    /// Whether the user opted into trustnet-test on Page 4.
    #[serde(default)]
    pub join_trustnet_test: bool,
    /// Whether the user requested a foundation grant on Page 5. The grant
    /// token itself is never persisted in plaintext — it is sent straight
    /// to the daemon's `dns.foundation_grant` IPC.
    #[serde(default)]
    pub requested_foundation_grant: bool,
    /// Mark when the wizard finished.
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub completed_at_unix: Option<i64>,
}

fn default_privacy() -> String { "private".into() }

fn hypermesh_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("HYPERMESH_HOME") {
        return PathBuf::from(custom);
    }
    dirs::home_dir()
        .map(|h| h.join(".hypermesh"))
        .unwrap_or_else(|| PathBuf::from(".hypermesh"))
}

fn identity_path() -> PathBuf { hypermesh_dir().join("identity.falcon") }
fn state_path() -> PathBuf { hypermesh_dir().join("wizard.json") }

pub async fn current_state() -> WizardState {
    let id_path = identity_path();
    let st_path = state_path();
    let identity_exists = id_path.exists();

    // Load persisted state, if any.
    let persisted: Option<WizardState> = match fs::read_to_string(&st_path).await {
        Ok(raw) => serde_json::from_str(&raw).ok(),
        Err(_) => None,
    };

    let mut state = persisted.unwrap_or_default();
    state.identity_path = id_path.display().to_string();
    state.state_path = st_path.display().to_string();
    // Show the wizard whenever there's no identity OR no completion record.
    state.should_show = !identity_exists || !state.completed;
    state
}

pub async fn set_privacy(mode: &str) -> anyhow::Result<()> {
    let mut s = current_state().await;
    s.privacy_mode = mode.to_string();
    persist(&s).await
}

pub async fn set_trustnet_test(opt_in: bool) -> anyhow::Result<()> {
    let mut s = current_state().await;
    s.join_trustnet_test = opt_in;
    persist(&s).await
}

pub async fn set_foundation_grant_requested(yes: bool) -> anyhow::Result<()> {
    let mut s = current_state().await;
    s.requested_foundation_grant = yes;
    persist(&s).await
}

pub async fn complete() -> anyhow::Result<WizardState> {
    let mut s = current_state().await;
    s.completed = true;
    s.completed_at_unix = Some(now_unix());
    s.should_show = false;
    persist(&s).await?;
    Ok(s)
}

async fn persist(state: &WizardState) -> anyhow::Result<()> {
    let dir = hypermesh_dir();
    fs::create_dir_all(&dir).await.ok();
    let path = state_path();
    let json = serde_json::to_string_pretty(state)?;
    fs::write(&path, json).await?;
    Ok(())
}

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
