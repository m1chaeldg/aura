//! Aura application executable entrypoint.

use anyhow::Result;
use aura::design::colors;
use aura::ui::AuraAppState;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Aura REST API Client v0.1.0 (GPUI + deno_core)");
    println!("Brand Accent: {} (Action Blue)", colors::PRIMARY);
    println!("Offline scripting, local filesystem sync, zero telemetry.");

    let mut state = AuraAppState::new();
    state.new_request();
    println!("Workspace initialized with {} tab(s).", state.tabs.len());

    Ok(())
}
