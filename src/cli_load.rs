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
    pub workspace_path: Option<PathBuf>,
    pub profile_name: Option<String>,
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
  -w, --workspace <dir>       Workspace root directory (uses workspace.json order & .aura/load-profiles)\n\
  -p, --profile <name>        Load saved workspace profile from .aura/load-profiles/<name>.json\n\
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
    let mut workspace_path: Option<PathBuf> = None;
    let mut profile_name: Option<String> = None;
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
            "-w" | "--workspace" => {
                workspace_path = Some(PathBuf::from(next_val(&mut i, arg)?));
            }
            "-p" | "--profile" => {
                profile_name = Some(next_val(&mut i, arg)?);
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

    if !show_help && target_path.is_none() && workspace_path.is_none() {
        return Err(anyhow!(
            "Missing target .aura file, collection folder, or --workspace (see `aura load --help`)"
        ));
    }

    let resolved_target = target_path
        .unwrap_or_else(|| workspace_path.clone().unwrap_or_default());

    Ok(LoadCliOptions {
        target_path: resolved_target,
        workspace_path,
        profile_name,
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
    let files = if target.is_file() {
        vec![target.to_path_buf()]
    } else {
        aura_sync::collect_ordered_request_paths(target, None)?
    };

    let mut reqs = Vec::with_capacity(files.len());
    for p in files {
        let content = std::fs::read_to_string(&p)
            .with_context(|| format!("Failed to read {}", p.display()))?;
        let req = aura::parser::parse_request(&content)
            .or_else(|_| aura_sync::toml_to_request(&content, None).map_err(|e| anyhow!("{e}")))
            .with_context(|| format!("Failed to parse {}", p.display()))?;
        reqs.push(req);
    }
    if reqs.is_empty() {
        return Err(anyhow!("No .aura or .toml request files found under {}", target.display()));
    }
    Ok(reqs)
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
    let ws_root = opts
        .workspace_path
        .as_deref()
        .unwrap_or(&opts.target_path);

    let mut cfg = opts.runner_config.clone();
    let mut ctx = TemplateContext::new();

    let requests = if let Some(ref prof_name) = opts.profile_name {
        let profile = aura_sync::load_load_profile(ws_root, prof_name)?;
        cfg = runner_config_from_workspace_profile(&profile, ws_root);

        if opts.env_path.is_none() {
            if let Some(ref env_name) = profile.environment_name {
                let envs = aura_sync::load_environments_from_dir(ws_root);
                if let Some(env_prof) = envs.into_iter().find(|e| e.1.name.eq_ignore_ascii_case(env_name)) {
                    for var in env_prof.1.variables {
                        if var.enabled {
                            ctx.environment_vars.insert(var.key, var.value);
                        }
                    }
                }
            }
        }

        let scope_folder = match &profile.target_scope {
            aura_sync::LoadTargetScope::Folder { relative_path } => {
                Some(PathBuf::from(relative_path))
            }
            _ => None,
        };
        let paths = aura_sync::collect_ordered_request_paths(ws_root, scope_folder.as_deref())?;
        let override_map: std::collections::HashMap<String, &aura_sync::LoadProfileRequestOverride> =
            profile
                .request_overrides
                .iter()
                .map(|o| (o.relative_path.clone(), o))
                .collect();

        let mut reqs = Vec::new();
        for p in paths {
            let rel = aura_sync::to_workspace_relative_string(ws_root, &p);
            if let Some(ov) = override_map.get(&rel) {
                if !ov.included {
                    continue;
                }
            }
            let content = tokio::fs::read_to_string(&p).await?;
            let mut req = aura::parser::parse_request(&content)
                .or_else(|_| aura_sync::toml_to_request(&content, None).map_err(|e| anyhow!("{e}")))?;
            if let Some(ov) = override_map.get(&rel) {
                let flow = req.flow.get_or_insert_with(Default::default);
                if let Some(w) = ov.weight_percent {
                    flow.weight_percent = Some(w);
                }
                if let Some(o) = ov.once_per_run {
                    flow.once_per_run = o;
                }
                if let Some(l) = ov.loop_count {
                    flow.loop_count = Some(l);
                }
                if let Some(ref c) = ov.condition_if {
                    flow.condition_if = Some(c.clone());
                }
                if let Some(ref w) = ov.loop_while {
                    flow.loop_while = Some(w.clone());
                }
            }
            reqs.push(req);
        }
        if reqs.is_empty() {
            return Err(anyhow!("No included requests found for profile '{prof_name}'"));
        }
        reqs
    } else {
        collect_aura_requests(&opts.target_path)?
    };

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
            cfg.concurrency
        );
    }

    let report = run_collection(requests, ctx, cfg, None).await;

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

fn runner_config_from_workspace_profile(
    p: &aura_sync::WorkspaceLoadProfile,
    ws_root: &Path,
) -> RunnerConfig {
    let think_time = match p.think_mode.as_str() {
        "Uniform" => ThinkTimeConfig::Uniform {
            min_ms: p.think_a.max(0.0) as u64,
            max_ms: p.think_b.max(0.0) as u64,
        },
        "Gaussian" => ThinkTimeConfig::Gaussian {
            mean_ms: p.think_a.max(0.0),
            stddev_ms: p.think_b.max(0.0),
        },
        _ => ThinkTimeConfig::None,
    };

    let sync_barrier = if p.sync_barrier_enabled {
        Some(SynchronizingTimerConfig {
            group_size: p.sync_barrier_group_size.max(2),
            timeout_ms: p.sync_barrier_timeout_ms.max(50),
        })
    } else {
        None
    };

    let data_feeder = if p.data_feeder_enabled {
        p.data_feeder_path.as_ref().map(|raw| DataFeederConfig {
            enabled: true,
            file_path: aura_sync::resolve_workspace_path(ws_root, raw)
                .display()
                .to_string(),
            sharing_mode: if p.data_sharing == "PerVU" {
                DataSharingMode::PerVU
            } else {
                DataSharingMode::AllVUs
            },
            eof_behavior: if p.data_eof == "StopVuOnEof" {
                DataEofBehavior::StopVuOnEof
            } else {
                DataEofBehavior::Recycle
            },
            ..Default::default()
        })
    } else {
        None
    };

    let sla_thresholds = if p.sla_p95_ms.is_some()
        || p.sla_p99_ms.is_some()
        || p.sla_error_rate_percent.is_some()
    {
        Some(SlaThresholds {
            max_p95_ms: p.sla_p95_ms,
            max_p99_ms: p.sla_p99_ms,
            max_error_rate_percent: p.sla_error_rate_percent,
        })
    } else {
        None
    };

    let (duration_secs, stages, ramp_down_secs) = match p.profile_mode.as_str() {
        "Duration" => (Some(p.duration_secs.unwrap_or(20.0)), Vec::new(), 0.0),
        "RampUpDown" => (
            Some(p.duration_secs.unwrap_or(20.0)),
            Vec::new(),
            p.ramp_down_secs.max(0.0),
        ),
        "SteppedStages" => (
            None,
            p.stages
                .iter()
                .map(|(v, s)| LoadStage::new(*v, *s))
                .collect(),
            0.0,
        ),
        _ => (None, Vec::new(), 0.0),
    };

    RunnerConfig {
        total_iterations: p.total_iterations.max(1),
        concurrency: p.concurrency.max(1),
        delay_ms: p.delay_ms,
        ramp_up_secs: p.ramp_up_secs.max(0.0),
        ramp_down_secs,
        duration_secs,
        stages,
        target_rps: p.target_rps,
        think_time,
        sync_barrier,
        weighted_routing: p.weighted_routing,
        data_feeder,
        sample_assertions: aura_core::assertions::PerformanceAssertionConfig {
            max_duration_ms: p.max_duration_ms,
            min_size_bytes: p.min_size_bytes,
            max_size_bytes: p.max_size_bytes,
        },
        sla_thresholds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_workspace_and_profile_flags() {
        let args: Vec<String> = vec![
            "--workspace",
            "./workspaces/space1",
            "--profile",
            "Smoke 25 VUs",
            "--quiet",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let opts = parse_load_cli_args(&args).unwrap();
        assert_eq!(
            opts.workspace_path,
            Some(PathBuf::from("./workspaces/space1"))
        );
        assert_eq!(opts.profile_name.as_deref(), Some("Smoke 25 VUs"));
        assert!(opts.quiet);
    }
}
