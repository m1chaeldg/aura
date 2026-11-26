//! Aura application executable entrypoint.

use anyhow::Result;
use aura::design::colors;
use aura::logging::init_system_logging;
use aura::ui::AuraWorkspaceView;
use gpui::{AppContext, WindowOptions};

fn main() -> Result<()> {
    // Increase main-thread stack to 8 MB for deeply nested GPUI render trees in debug builds.
    // The default Windows stack (1 MB) overflows with the workspace view's builder chain depth.
    const STACK_SIZE: usize = 8 * 1024 * 1024;
    let builder = std::thread::Builder::new()
        .name("aura-main".into())
        .stack_size(STACK_SIZE);
    let handler = builder.spawn(move || -> Result<()> { aura_main() })?;
    handler.join().unwrap()
}

fn aura_main() -> Result<()> {
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

    let app = gpui_kit::application();
    app.run(|cx| {
        aura_ui::init_kit(cx);
        let window_options = WindowOptions::default();

        let _ = gpui_kit::open_window(window_options, cx, |window, cx| {
            let view = cx.new(AuraWorkspaceView::new);
            cx.new(|cx| gpui_kit::component::Root::new(view, window, cx))
        });
    });

    tracing::info!("Aura application shutting down.");
    Ok(())
}
