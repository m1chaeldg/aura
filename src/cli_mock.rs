//! Headless Mock Server CLI (`aura mock ...`) for local development, CI, and production containers.

use anyhow::{anyhow, Context, Result};
use aura::logging::init_system_logging;
use aura_core::mock::{MockServerConfigFile, MockUnmatchedPolicy};
use aura_mock::MockServer;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct MockCliOptions {
    pub config_path: Option<PathBuf>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub max_body_bytes: Option<usize>,
    pub passthrough_url: Option<String>,
    pub not_found: bool,
    pub show_help: bool,
}

pub fn mock_cli_help_text() -> &'static str {
    "Usage: aura mock [OPTIONS]\n\
\n\
Options:\n\
  -c, --config <path.json>    Path to portable MockServerConfigFile JSON\n\
      --host <ip>             Bind host address (default: 127.0.0.1, use 0.0.0.0 for Docker/prod)\n\
  -p, --port <port>           Bind TCP port (default: 18080)\n\
      --max-body-bytes <n>    Maximum allowed request body size in bytes\n\
      --passthrough <url>     Forward unmatched requests to upstream base URL\n\
      --not-found             Return 404 JSON for unmatched requests\n\
  -h, --help                  Show this help message"
}

pub fn parse_mock_cli_args(args: &[String]) -> Result<MockCliOptions> {
    let mut opts = MockCliOptions::default();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                opts.show_help = true;
                return Ok(opts);
            }
            "-c" | "--config" => {
                i += 1;
                let val = args.get(i).ok_or_else(|| anyhow!("Missing value for --config"))?;
                opts.config_path = Some(PathBuf::from(val));
            }
            "--host" => {
                i += 1;
                let val = args.get(i).ok_or_else(|| anyhow!("Missing value for --host"))?;
                opts.host = Some(val.clone());
            }
            "-p" | "--port" => {
                i += 1;
                let val = args.get(i).ok_or_else(|| anyhow!("Missing value for --port"))?;
                opts.port = Some(val.parse::<u16>().context("Invalid --port number")?);
            }
            "--max-body-bytes" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| anyhow!("Missing value for --max-body-bytes"))?;
                opts.max_body_bytes = Some(
                    val.parse::<usize>()
                        .context("Invalid --max-body-bytes integer")?,
                );
            }
            "--passthrough" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| anyhow!("Missing value for --passthrough"))?;
                opts.passthrough_url = Some(val.clone());
            }
            "--not-found" => {
                opts.not_found = true;
            }
            other if !other.starts_with('-') && opts.config_path.is_none() => {
                opts.config_path = Some(PathBuf::from(other));
            }
            other => return Err(anyhow!("Unknown option for `aura mock`: {other}")),
        }
        i += 1;
    }
    Ok(opts)
}

pub fn run_headless_mock_sync(args: &[String]) -> i32 {
    let opts = match parse_mock_cli_args(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Error: {e}\n\n{}", mock_cli_help_text());
            return 2;
        }
    };

    if opts.show_help {
        println!("{}", mock_cli_help_text());
        return 0;
    }

    let _logging_guard = init_system_logging(None).ok();

    let mut cfg = if let Some(path) = &opts.config_path {
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read mock config {}: {e}", path.display());
                return 1;
            }
        };
        match MockServerConfigFile::from_json_str(&raw) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to parse mock config {}: {e}", path.display());
                return 1;
            }
        }
    } else {
        MockServerConfigFile::default()
    };

    if let Some(host) = opts.host {
        cfg.host = host;
    }
    if let Some(port) = opts.port {
        cfg.port = port;
    }
    if let Some(max) = opts.max_body_bytes {
        cfg.max_body_bytes = max;
    }
    if let Some(url) = opts.passthrough_url {
        cfg.unmatched = MockUnmatchedPolicy::ProxyPassthrough {
            target_base_url: url,
        };
    } else if opts.not_found {
        cfg.unmatched = MockUnmatchedPolicy::NotFound;
    }

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to initialize Tokio runtime: {e}");
            return 1;
        }
    };

    rt.block_on(async move {
        let server = MockServer::from_config(cfg);
        match server.start().await {
            Ok(handle) => {
                println!(
                    "Aura Mock Server running at http://{} (press Ctrl+C to stop)",
                    handle.listen_addr
                );
                let _ = tokio::signal::ctrl_c().await;
                println!("\nShutting down mock server...");
                handle.stop();
                0
            }
            Err(e) => {
                eprintln!("Failed to start mock server: {e}");
                1
            }
        }
    })
}
