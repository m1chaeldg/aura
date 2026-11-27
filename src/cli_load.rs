//! Headless Load CLI (`aura load ...`) — executes JMeter load profiles without GPUI rendering overhead.

use anyhow::{anyhow, Context, Result};
use aura::model::{AuraRequest, ThinkTimeConfig};
use aura::reporting::{
    compute_aggregate_report, export_samples_json, export_samples_jtl_csv, generate_html_report,
};
use aura::runner::{
    run_collection, LoadStage, RunnerConfig, RunnerReport, SlaThresholds, SynchronizingTimerConfig,
};
use aura::templating::TemplateContext;
use aura_core::data_feeder::{DataEofBehavior, DataFeederConfig, DataSharingMode};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LoadCliOptions {
    pub target_path: PathBuf,
    pub env_path: Option<PathBuf>,
    pub runner_config: RunnerConfig,
    pub export_csv: Option<PathBuf>,
    pub export_json: Option<PathBuf>,
    pub export_html: Option<PathBuf>,
    pub quiet: bool,
    pub show_help: bool,
}

pub fn load_cli_help_text() -> &'static str {
    "Usage: aura load <collection_dir_or_file.aura> [OPTIONS]\n\
\n\
Options:\n\
  -c, --collection <path>     Target .aura request file or collection directory\n\
  -e, --env <path.toml>       Environment TOML profile file\n\
  -u, --vus, --concurrency N  Virtual user concurrency (default: 2)\n\
  -n, --iterations N          Total iterations for fixed mode (default: 10)\n\
  -d, --duration <secs>       Time-boxed soak/stress duration in seconds\n\
      --ramp-up <secs>        Ramp-up period in seconds\n\
      --ramp-down <secs>      Ramp-down period in seconds\n\
      --stages <v:s,v:s,...>  Multi-stage load schedule (e.g. 10:30,100:60,10:30)\n\
      --target-rps <rps>      Constant throughput shaping cap (requests/sec)\n\
      --think-uniform <a:b>   Uniform random think-time range in ms (e.g. 200:800)\n\
      --think-gaussian <m:s>  Gaussian think-time mean:stddev in ms (e.g. 400:100)\n\
      --sync-barrier <n[:ms]> Synchronizing Rendezvous Barrier group size & optional timeout ms\n\
      --weighted              Distribute requests using [flow] weight_percent weights\n\
      --data <file>           CSV or JSON data feeder file\n\
      --sharing all|vu        Data feeder sharing mode (default: all)\n\
      --eof recycle|stop      Data feeder EOF behavior (default: recycle)\n\
      --max-duration <ms>     Per-sample max duration assertion (ms)\n\
      --min-size <bytes>      Per-sample min response size (bytes)\n\
      --max-size <bytes>      Per-sample max response size (bytes)\n\
      --sla-p95 <ms>          Global CI SLA gate for p95 latency (ms)\n\
      --sla-p99 <ms>          Global CI SLA gate for p99 latency (ms)\n\
      --sla-error <pct>       Global CI SLA gate for max error rate (%)\n\
      --export-csv <path>     Export raw JTL-style CSV samples\n\
      --export-json <path>    Export full JSON report + time series\n\
      --export-html <path>    Generate standalone HTML summary report\n\
  -q, --quiet                 Suppress per-sample terminal output\n\
  -h, --help                  Show this help message"
}

pub fn parse_load_cli_args(args: &[String]) -> Result<LoadCliOptions> {
    let mut target_path: Option<PathBuf> = None;
    let mut env_path: Option<PathBuf> = None;
    let mut cfg = RunnerConfig::default();
    let mut data_cfg = DataFeederConfig::default();
    let mut sla = SlaThresholds::default();
    let mut has_sla = false;
    let mut export_csv: Option<PathBuf> = None;
    let mut export_json: Option<PathBuf> = None;
    let mut export_html: Option<PathBuf> = None;
    let mut quiet = false;
    let mut show_help = false;

    let mut i = 0usize;
    while i < args.len() {
        let arg = args[i].as_str();
        let next_val = |idx: &mut usize, flag: &str| -> Result<String> {
            *idx += 1;
            args.get(*idx)
                .cloned()
                .ok_or_else(|| anyhow!("Missing value for {flag}"))
        };

        match arg {
            "-h" | "--help" => show_help = true,
            "-q" | "--quiet" => quiet = true,
            "--weighted" => cfg.weighted_routing = true,
            "-c" | "--collection" => {
                target_path = Some(PathBuf::from(next_val(&mut i, arg)?));
            }
            "-e" | "--env" => {
                env_path = Some(PathBuf::from(next_val(&mut i, arg)?));
            }
            "-u" | "--vus" | "--concurrency" => {
                cfg.concurrency = next_val(&mut i, arg)?.parse::<usize>().context("invalid --vus")?;
            }
            "-n" | "--iterations" => {
                cfg.total_iterations = next_val(&mut i, arg)?.parse::<usize>().context("invalid --iterations")?;
            }
            "-d" | "--duration" => {
                cfg.duration_secs = Some(next_val(&mut i, arg)?.parse::<f64>().context("invalid --duration")?);
            }
            "--ramp-up" => {
                cfg.ramp_up_secs = next_val(&mut i, arg)?.parse::<f64>().context("invalid --ramp-up")?;
            }
            "--ramp-down" => {
                cfg.ramp_down_secs = next_val(&mut i, arg)?.parse::<f64>().context("invalid --ramp-down")?;
            }
            "--stages" => {
                let raw = next_val(&mut i, arg)?;
                cfg.stages = parse_stages_spec(&raw)?;
            }
            "--target-rps" => {
                cfg.target_rps = Some(next_val(&mut i, arg)?.parse::<f64>().context("invalid --target-rps")?);
            }
            "--think-uniform" => {
                let raw = next_val(&mut i, arg)?;
                let (a, b) = raw
                    .split_once(':')
                    .ok_or_else(|| anyhow!("Expected min:max for --think-uniform"))?;
                cfg.think_time = ThinkTimeConfig::Uniform {
                    min_ms: a.trim().parse()?,
                    max_ms: b.trim().parse()?,
                };
            }
            "--think-gaussian" => {
                let raw = next_val(&mut i, arg)?;
                let (m, s) = raw
                    .split_once(':')
                    .ok_or_else(|| anyhow!("Expected mean:stddev for --think-gaussian"))?;
                cfg.think_time = ThinkTimeConfig::Gaussian {
                    mean_ms: m.trim().parse()?,
                    stddev_ms: s.trim().parse()?,
                };
            }
            "--sync-barrier" => {
                let raw = next_val(&mut i, arg)?;
                if let Some((n, to)) = raw.split_once(':') {
                    cfg.sync_barrier = Some(SynchronizingTimerConfig {
                        group_size: n.trim().parse()?,
                        timeout_ms: to.trim().parse()?,
                    });
                } else {
                    cfg.sync_barrier = Some(SynchronizingTimerConfig {
                        group_size: raw.trim().parse()?,
                        timeout_ms: 5000,
                    });
                }
            }
            "--data" => {
                data_cfg.enabled = true;
                data_cfg.file_path = next_val(&mut i, arg)?;
            }
            "--sharing" => {
                let v = next_val(&mut i, arg)?;
                data_cfg.sharing_mode = match v.to_ascii_lowercase().as_str() {
                    "vu" | "per-vu" | "per_vu" => DataSharingMode::PerVU,
                    _ => DataSharingMode::AllVUs,
                };
            }
            "--eof" => {
                let v = next_val(&mut i, arg)?;
                data_cfg.eof_behavior = match v.to_ascii_lowercase().as_str() {
                    "stop" | "stop-vu" => DataEofBehavior::StopVuOnEof,
                    _ => DataEofBehavior::Recycle,
                };
            }
            "--max-duration" => {
                cfg.sample_assertions.max_duration_ms = Some(next_val(&mut i, arg)?.parse()?);
            }
            "--min-size" => {
                cfg.sample_assertions.min_size_bytes = Some(next_val(&mut i, arg)?.parse()?);
            }
            "--max-size" => {
                cfg.sample_assertions.max_size_bytes = Some(next_val(&mut i, arg)?.parse()?);
            }
            "--sla-p95" => {
                sla.max_p95_ms = Some(next_val(&mut i, arg)?.parse()?);
                has_sla = true;
            }
            "--sla-p99" => {
                sla.max_p99_ms = Some(next_val(&mut i, arg)?.parse()?);
                has_sla = true;
            }
            "--sla-error" => {
                sla.max_error_rate_percent = Some(next_val(&mut i, arg)?.parse()?);
                has_sla = true;
            }
            "--export-csv" => export_csv = Some(PathBuf::from(next_val(&mut i, arg)?)),
            "--export-json" => export_json = Some(PathBuf::from(next_val(&mut i, arg)?)),
            "--export-html" => export_html = Some(PathBuf::from(next_val(&mut i, arg)?)),
            other if !other.starts_with('-') && target_path.is_none() => {
                target_path = Some(PathBuf::from(other));
            }
            unknown => return Err(anyhow!("Unknown CLI argument: {unknown}")),
        }
        i += 1;
    }

    if data_cfg.enabled {
        cfg.data_feeder = Some(data_cfg);
    }
    if has_sla {
        cfg.sla_thresholds = Some(sla);
    }

    if !show_help && target_path.is_none() {
        return Err(anyhow!("Missing target .aura file or collection folder (see `aura load --help`)"));
    }

    Ok(LoadCliOptions {
        target_path: target_path.unwrap_or_default(),
        env_path,
        runner_config: cfg,
        export_csv,
        export_json,
        export_html,
        quiet,
        show_help,
    })
}

fn parse_stages_spec(raw: &str) -> Result<Vec<LoadStage>> {
    let mut stages = Vec::new();
    for seg in raw.split(',') {
        let s = seg.trim();
        if s.is_empty() {
            continue;
        }
        let (vus_s, dur_s) = s
            .split_once(':')
            .ok_or_else(|| anyhow!("Invalid stage `{s}`, expected <vus>:<secs>"))?;
        stages.push(LoadStage::new(vus_s.trim().parse()?, dur_s.trim().parse()?));
    }
    if stages.is_empty() {
        return Err(anyhow!("At least one stage required for --stages"));
    }
    Ok(stages)
}

pub fn collect_aura_requests(target: &Path) -> Result<Vec<AuraRequest>> {
    if !target.exists() {
        return Err(anyhow!("Path does not exist: {}", target.display()));
    }
    let mut files = Vec::new();
    if target.is_file() {
        files.push(target.to_path_buf());
    } else {
        collect_aura_files_recursive(target, &mut files)?;
        files.sort();
    }

    let mut reqs = Vec::with_capacity(files.len());
    for p in files {
        let content = std::fs::read_to_string(&p)
            .with_context(|| format!("Failed to read {}", p.display()))?;
        let req = aura::parser::parse_request(&content)
            .map_err(|e| anyhow!("Failed to parse {}: {e}", p.display()))?;
        reqs.push(req);
    }
    if reqs.is_empty() {
        return Err(anyhow!("No .aura request files found under {}", target.display()));
    }
    Ok(reqs)
}

fn collect_aura_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name != ".git" && name != "target" {
                collect_aura_files_recursive(&path, out)?;
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("aura") {
            out.push(path);
        }
    }
    Ok(())
}

pub fn run_headless_load_sync(args: &[String]) -> i32 {
    let opts = match parse_load_cli_args(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Error: {e:#}");
            return 1;
        }
    };

    if opts.show_help {
        println!("{}", load_cli_help_text());
        return 0;
    }

    let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to initialize async runtime: {e}");
            return 1;
        }
    };

    match rt.block_on(execute_headless_load(opts)) {
        Ok(report) => {
            if report.sla_passed {
                0
            } else {
                2
            }
        }
        Err(e) => {
            eprintln!("Load test failed: {e:#}");
            1
        }
    }
}

pub async fn execute_headless_load(opts: LoadCliOptions) -> Result<RunnerReport> {
    let requests = collect_aura_requests(&opts.target_path)?;
    let mut ctx = TemplateContext::new();
    if let Some(ref env_file) = opts.env_path {
        let raw_env = tokio::fs::read_to_string(env_file).await?;
        let profile = aura_sync::toml_to_environment(&raw_env, None)?;
        for var in profile.variables {
            if var.enabled {
                ctx.environment_vars.insert(var.key, var.value);
            }
        }
    }

    if !opts.quiet {
        println!(
            "Running headless load test on {} ({} request(s), concurrency={})...",
            opts.target_path.display(),
            requests.len(),
            opts.runner_config.concurrency
        );
    }

    let report = run_collection(requests, ctx, opts.runner_config.clone(), None).await;

    if !opts.quiet {
        print_cli_summary(&report);
    }

    if let Some(ref csv_path) = opts.export_csv {
        tokio::fs::write(csv_path, export_samples_jtl_csv(&report.execution_log)).await?;
    }
    if let Some(ref json_path) = opts.export_json {
        tokio::fs::write(json_path, export_samples_json(&report)?).await?;
    }
    if let Some(ref html_path) = opts.export_html {
        let title = opts
            .target_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Aura Load Report");
        tokio::fs::write(html_path, generate_html_report(title, &report)).await?;
    }

    Ok(report)
}

fn print_cli_summary(report: &RunnerReport) {
    println!(
        "\n=== Aura Load Test Summary ===\nSamples: {} (Pass: {}, Fail: {}, Error: {:.2}%)\nThroughput: {:.2} RPS | Peak VUs: {} | Duration: {} ms\nLatency (ms): min={} mean={:.1} p50={} p90={} p95={} p99={} max={}",
        report.total_requests,
        report.successful_requests,
        report.failed_requests,
        report.error_rate_percent,
        report.requests_per_second,
        report.peak_vus,
        report.total_duration_ms,
        report.min_latency_ms,
        report.mean_latency_ms,
        report.p50_latency_ms,
        report.p90_latency_ms,
        report.p95_latency_ms,
        report.p99_latency_ms,
        report.max_latency_ms
    );

    let aggs = compute_aggregate_report(&report.execution_log, report.total_duration_ms);
    if !aggs.is_empty() {
        println!("\n{:<28} {:<7} {:>7} {:>8} {:>7} {:>7} {:>7} {:>7} {:>7} {:>8} {:>8}", "Endpoint", "Method", "Samples", "Avg(ms)", "Med", "p95", "p99", "Err%", "RPS", "KB/s", "Max");
        for a in aggs {
            println!(
                "{:<28} {:<7} {:>7} {:>8.1} {:>7} {:>7} {:>7} {:>6.1}% {:>7.2} {:>8.2} {:>7}",
                a.request_name,
                a.method,
                a.samples,
                a.avg_ms,
                a.median_ms,
                a.p95_ms,
                a.p99_ms,
                a.error_percent,
                a.rps,
                a.kb_per_sec,
                a.max_ms
            );
        }
    }

    if !report.sla_passed {
        println!("\nSLA GATES: FAILED");
        for v in &report.sla_violations {
            println!("  - {}: {} (expected {})", v.metric, v.actual, v.threshold);
        }
    } else {
        println!("\nSLA GATES: PASSED");
    }
}
