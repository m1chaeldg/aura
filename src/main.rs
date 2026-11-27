//! Aura application executable entrypoint.

use anyhow::Result;
use aura::logging::init_system_logging;
use aura::ui::AuraWorkspaceView;
use gpui::WindowOptions;
use gpui_kit::AppContext as _;

mod cli_load;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("load") {
        let code = cli_load::run_headless_load_sync(&args[2..]);
        std::process::exit(code);
    }

    // Increase main-thread stack to 8 MB for deeply nested UI hierarchies.
    // The default Windows stack (1 MB) overflows with deeply nested UI hierarchies.
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

    tracing::info!("Aura REST API Client v0.1.0 starting up...");
    tracing::info!("Launching Desktop GUI Window...");

    // Initialize global multi-thread Tokio runtime for background async services
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _tokio_guard = tokio_rt.enter();

    let app = gpui_kit::application().with_assets(gpui_kit::assets::AllAssets);
    app.run(|cx| {
        aura_ui::init_kit(cx);
        let window_options = WindowOptions::default();

        let _ = gpui_kit::open_window(window_options, cx, |_, cx| cx.new(AuraWorkspaceView::new));
    });

    tracing::info!("Aura application shutting down.");
    Ok(())
}
