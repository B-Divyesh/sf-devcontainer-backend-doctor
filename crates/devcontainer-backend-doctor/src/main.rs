use clap::{Args, Parser, Subcommand};
use devcontainer_backend_doctor::{Backend, DoctorError, Report, Severity, inspect};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "devcontainer-backend-doctor",
    version,
    about = "Predict devcontainer and Compose breakage before switching Mac container backends",
    long_about = "Statically inspects devcontainer.json and Compose files for backend-specific mounts, privileges, networking, architecture, GPU, and secret requirements. Backend probes are read-only and run only with --probe.",
    after_help = "Examples:\n  devcontainer-backend-doctor check . --backend podman\n  devcontainer-backend-doctor check . --backend apple-container --json\n  devcontainer-backend-doctor check ./service --backend orbstack --probe\n\nExit codes: 0 clean, 1 threshold met, 2 invalid project, 3 probe failed"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Inspect a project without starting containers
    Check(CheckArgs),
}

#[derive(Args, Debug)]
struct CheckArgs {
    /// Project directory or specific devcontainer/Compose file
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Backend to evaluate: docker-desktop, podman, orbstack, apple-container
    #[arg(long, value_parser = parse_backend)]
    backend: Backend,

    /// Run a short, read-only version/status probe for the selected backend
    #[arg(long)]
    probe: bool,

    /// Emit the versioned report as JSON
    #[arg(long)]
    json: bool,

    /// Exit 1 at this severity or higher: info, warning, error
    #[arg(long, default_value = "error", value_parser = parse_severity)]
    fail_on: Severity,
}

fn parse_backend(value: &str) -> Result<Backend, String> {
    value.parse()
}

fn parse_severity(value: &str) -> Result<Severity, String> {
    value.parse()
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check(args) => run_check(args),
    }
}

fn run_check(args: CheckArgs) -> ExitCode {
    match inspect(&args.path, args.backend, args.probe) {
        Ok(report) => {
            if args.json {
                match serde_json::to_string_pretty(&report) {
                    Ok(json) => println!("{json}"),
                    Err(error) => {
                        eprintln!("doctor: could not serialize report: {error}");
                        return ExitCode::from(2);
                    }
                }
            } else {
                print_human(&report);
            }
            if report
                .findings
                .iter()
                .any(|finding| finding.severity >= args.fail_on)
            {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(DoctorError::Probe(message)) => {
            if args.json {
                println!(
                    "{{\"schema_version\":1,\"error\":\"probe_failed\",\"message\":{}}}",
                    serde_json::to_string(&message)
                        .unwrap_or_else(|_| "\"probe failed\"".to_owned())
                );
            } else {
                eprintln!("doctor: {message}");
                eprintln!(
                    "hint: omit --probe for static analysis, or start/install the selected backend"
                );
            }
            ExitCode::from(3)
        }
        Err(error) => {
            if args.json {
                println!(
                    "{{\"schema_version\":1,\"error\":\"invalid_project\",\"message\":{}}}",
                    serde_json::to_string(&error.to_string())
                        .unwrap_or_else(|_| "\"invalid project\"".to_owned())
                );
            } else {
                eprintln!("doctor: {error}");
                eprintln!(
                    "hint: pass a project directory, devcontainer.json, or Compose YAML file"
                );
            }
            ExitCode::from(2)
        }
    }
}

fn print_human(report: &Report) {
    println!("Devcontainer Backend Doctor");
    println!("Backend  : {}", report.backend);
    println!("Project  : {}", report.project_root);
    println!("Files    : {}", report.project_files.join(", "));
    println!("Host arch: {}", report.host_architecture);
    if let Some(probe) = &report.probe {
        println!(
            "Probe    : {} ({})",
            if probe.healthy {
                "healthy"
            } else {
                "unhealthy"
            },
            probe.version.as_deref().unwrap_or("version unknown")
        );
    } else {
        println!("Probe    : skipped (use --probe to opt in)");
    }
    println!();
    if report.findings.is_empty() {
        println!("✓ No known compatibility risks found.");
    }
    for finding in &report.findings {
        let marker = match finding.severity {
            Severity::Error => "✕",
            Severity::Warning => "!",
            Severity::Info => "i",
        };
        let location = match finding.line {
            Some(line) => format!("{}:{line}", finding.source),
            None => finding.source.clone(),
        };
        println!("{marker} {} [{}]", finding.title, finding.rule_id);
        println!("  {} · {location}", finding.severity.as_str());
        println!("  {}", finding.message);
        println!("  Evidence: {}", finding.evidence);
        println!("  Fix: {}", finding.remediation);
        println!();
    }
    println!(
        "Result: {} error(s), {} warning(s), {} info",
        report.summary.errors, report.summary.warnings, report.summary.info
    );
    println!(
        "Verdict: {}",
        if report.compatible {
            "no known blockers"
        } else {
            "switch blocked by known incompatibilities"
        }
    );
}
