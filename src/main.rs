//! Aura application executable entrypoint.

use anyhow::Result;
use aura::design::colors;
use aura::logging::init_system_logging;
use aura::ui::AuraWorkspaceView;
use gpui::{AppContext, Application, WindowOptions};

fn main() -> Result<()> {
    // Initialize asynchronous structured system logging & rolling file diagnostics
    let _logging_guard = init_system_logging(None)?;

    tracing::info!("Aura REST API Client v0.1.0 (GPUI + deno_core) starting up...");
    tracing::info!("Brand Accent: {} (Action Blue)", colors::PRIMARY);
    tracing::info!("Launching Desktop GUI Window...");

    // Initialize global multi-thread Tokio runtime for background async services
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _tokio_guard = tokio_rt.enter();

    let app = Application::new();
    app.run(|cx| {
        let window_options = WindowOptions::default();

        let _ = cx.open_window(window_options, |_window, cx| {
            cx.new(|cx| AuraWorkspaceView::new(cx))
        });
    });

    tracing::info!("Aura application shutting down.");
    Ok(())
}
