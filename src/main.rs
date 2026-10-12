//! Aura application executable entrypoint.

use anyhow::Result;
use aura::design::colors;
use aura::ui::AuraWorkspaceView;
use gpui::{AppContext, Application, WindowOptions};

fn main() -> Result<()> {
    println!("Aura REST API Client v0.1.0 (GPUI + deno_core)");
    println!("Brand Accent: {} (Action Blue)", colors::PRIMARY);
    println!("Launching Desktop GUI Window...");

    let app = Application::new();
    app.run(|cx| {
        let window_options = WindowOptions::default();

        let _ = cx.open_window(window_options, |_window, cx| {
            cx.new(|_cx| AuraWorkspaceView::new())
        });
    });

    Ok(())
}
