//! Static compatibility inspection for devcontainers and Compose projects.
//!
//! The library exposes a deliberately small API: discover a project and run
//! an inspection against one selected backend. It never starts containers or
//! changes daemon state.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml_ng::Value as YamlValue;
use std::collections::{BTreeSet, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};

/// A supported local container backend.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Backend {
    DockerDesktop,
    Podman,
    Orbstack,
    AppleContainer,
}

impl Backend {
    pub const ALL: [Self; 4] = [
        Self::DockerDesktop,
        Self::Podman,
        Self::Orbstack,
        Self::AppleContainer,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::DockerDesktop => "docker-desktop",
            Self::Podman => "podman",
            Self::Orbstack => "orbstack",
            Self::AppleContainer => "apple-container",
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Backend {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "docker-desktop" | "docker" => Ok(Self::DockerDesktop),
            "podman" => Ok(Self::Podman),
            "orbstack" => Ok(Self::Orbstack),
            "apple-container" | "apple" => Ok(Self::AppleContainer),
            _ => Err(format!(
                "unknown backend '{value}'; expected docker-desktop, podman, orbstack, or apple-container"
            )),
        }
    }
}

/// Finding severity, ordered from informational to incompatible.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl FromStr for Severity {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "info" => Ok(Self::Info),
            "warning" | "warn" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            _ => Err(format!("unknown severity '{value}'")),
        }
    }
}

/// One actionable compatibility result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub title: String,
    pub source: String,
    pub line: Option<usize>,
    pub evidence: String,
    pub message: String,
    pub remediation: String,
}

/// Summary counts for scripting.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
}

/// A read-only backend probe result. Output is normalized rather than copied
/// verbatim so environment or credential values cannot leak into reports.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeResult {
    pub command: String,
    pub available: bool,
    pub healthy: bool,
    pub version: Option<String>,
    pub architecture: Option<String>,
    pub note: String,
}

/// Versioned, machine-readable inspection report.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Report {
    pub schema_version: u8,
    pub backend: Backend,
    pub host_architecture: String,
    pub project_root: String,
    pub project_files: Vec<String>,
    pub summary: Summary,
    pub compatible: bool,
    pub findings: Vec<Finding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probe: Option<ProbeResult>,
}

#[derive(Debug)]
pub enum DoctorError {
    Input(String),
    Parse { path: PathBuf, message: String },
    Probe(String),
}

impl fmt::Display for DoctorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(message) | Self::Probe(message) => f.write_str(message),
            Self::Parse { path, message } => {
                write!(f, "could not parse {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for DoctorError {}

#[derive(Debug)]
enum ProjectFile {
    Devcontainer {
        path: PathBuf,
        raw: String,
        value: JsonValue,
    },
    Compose {
        path: PathBuf,
        raw: String,
        value: YamlValue,
    },
}

/// Inspect `input` against `backend`. When `should_probe` is false, no process
/// is executed and the report contains no probe section.
pub fn inspect(
    input: impl AsRef<Path>,
    backend: Backend,
    should_probe: bool,
) -> Result<Report, DoctorError> {
    let input = input.as_ref();
    let (root, files) = discover(input)?;
    let mut findings = Vec::new();
    for file in &files {
        match file {
            ProjectFile::Devcontainer { path, raw, value } => {
                inspect_devcontainer(path, raw, value, backend, &mut findings)
            }
            ProjectFile::Compose { path, raw, value } => {
                inspect_compose(path, raw, value, backend, &mut findings)
            }
        }
    }

    let probe = should_probe.then(|| probe_backend(backend)).transpose()?;
    if let Some(probe_result) = &probe {
        apply_version_rules(backend, probe_result, &mut findings);
    }
    deduplicate(&mut findings);
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });
    let summary = summarize(&findings);
    let project_files = files
        .iter()
        .map(|file| match file {
            ProjectFile::Devcontainer { path, .. } | ProjectFile::Compose { path, .. } => {
                display_path(path, &root)
            }
        })
        .collect();

    Ok(Report {
        schema_version: 1,
        backend,
        host_architecture: normalize_arch(std::env::consts::ARCH).to_owned(),
        project_root: root.display().to_string(),
        project_files,
        compatible: summary.errors == 0,
        summary,
        findings,
        probe,
    })
}

fn discover(input: &Path) -> Result<(PathBuf, Vec<ProjectFile>), DoctorError> {
    if !input.exists() {
        return Err(DoctorError::Input(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    let root = if input.is_dir() {
        input.to_path_buf()
    } else {
        input
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    };
    let mut paths = Vec::new();
    if input.is_file() {
        paths.push(input.to_path_buf());
    } else {
        for relative in [
            ".devcontainer/devcontainer.json",
            "devcontainer.json",
            "compose.yml",
            "compose.yaml",
            "docker-compose.yml",
            "docker-compose.yaml",
        ] {
            let candidate = root.join(relative);
            if candidate.is_file() {
                paths.push(candidate);
            }
        }
    }
    if paths.is_empty() {
        return Err(DoctorError::Input(format!(
            "no devcontainer.json or Compose file found under {}",
            input.display()
        )));
    }

    let mut seen = BTreeSet::new();
    let mut files = Vec::new();
    let mut cursor = 0;
    while cursor < paths.len() {
        let path = paths[cursor].clone();
        cursor += 1;
        let canonical_key = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !seen.insert(canonical_key) {
            continue;
        }
        let raw = fs::read_to_string(&path).map_err(|error| {
            DoctorError::Input(format!("could not read {}: {error}", path.display()))
        })?;
        if is_json_file(&path) {
            let value: JsonValue = json5::from_str(&raw).map_err(|error| DoctorError::Parse {
                path: path.clone(),
                message: error.to_string(),
            })?;
            for linked in linked_compose_files(&value) {
                let base = path.parent().unwrap_or(&root);
                let linked_path = base.join(linked);
                if linked_path.is_file() {
                    paths.push(linked_path);
                }
            }
            files.push(ProjectFile::Devcontainer { path, raw, value });
        } else if is_compose_file(&path) {
            let value: YamlValue =
                serde_yaml_ng::from_str(&raw).map_err(|error| DoctorError::Parse {
                    path: path.clone(),
                    message: error.to_string(),
                })?;
            files.push(ProjectFile::Compose { path, raw, value });
        } else {
            return Err(DoctorError::Input(format!(
                "unsupported input {}; expected devcontainer.json or a .yml/.yaml Compose file",
                path.display()
            )));
        }
    }
    Ok((root, files))
}

fn is_json_file(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("devcontainer.json")
        || path.extension().and_then(|ext| ext.to_str()) == Some("json")
}

fn is_compose_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yml" | "yaml")
    )
}

fn linked_compose_files(value: &JsonValue) -> Vec<PathBuf> {
    let Some(linked) = value.get("dockerComposeFile") else {
        return Vec::new();
    };
    match linked {
        JsonValue::String(path) => vec![PathBuf::from(path)],
        JsonValue::Array(paths) => paths
            .iter()
            .filter_map(JsonValue::as_str)
            .map(PathBuf::from)
            .collect(),
        _ => Vec::new(),
    }
}

fn inspect_devcontainer(
    path: &Path,
    raw: &str,
    value: &JsonValue,
    backend: Backend,
    findings: &mut Vec<Finding>,
) {
    if backend == Backend::AppleContainer {
        push_finding(
            findings,
            "APPLE-DEVCONTAINER-INTEGRATION",
            Severity::Error,
            "Devcontainer launch is not supported",
            path,
            line_for(raw, "dockerComposeFile").or(Some(1)),
            "A devcontainer configuration is present.",
            "VS Code Dev Containers does not currently offer a stable Apple Container backend path.",
            "Keep a Docker-compatible backend for this project or validate editor support before switching.",
        );
    }

    if value.get("privileged").and_then(JsonValue::as_bool) == Some(true) {
        privileged_finding(path, raw, backend, findings);
    }
    if let Some(capabilities) = value.get("capAdd").and_then(JsonValue::as_array) {
        if !capabilities.is_empty() && backend != Backend::DockerDesktop {
            push_finding(
                findings,
                "SEC-CAP-ADD",
                Severity::Warning,
                "Added Linux capabilities need review",
                path,
                line_for(raw, "capAdd"),
                &format!(
                    "The devcontainer adds {} Linux capability entries.",
                    capabilities.len()
                ),
                "Capability availability differs across VM-backed and rootless runtimes.",
                "Remove capabilities that are not required, then test the remaining names on the selected backend.",
            );
        }
    }

    if let Some(mounts) = value.get("mounts").and_then(JsonValue::as_array) {
        for mount in mounts.iter().filter_map(JsonValue::as_str) {
            inspect_mount(path, raw, mount, backend, findings);
        }
    }
    if let Some(args) = value.get("runArgs").and_then(JsonValue::as_array) {
        let values: Vec<&str> = args.iter().filter_map(JsonValue::as_str).collect();
        if values.iter().any(|value| value.contains("network=host")) {
            host_network_finding(path, raw, backend, findings);
        }
        if values
            .iter()
            .any(|value| value.contains("--gpus") || value.contains("nvidia"))
        {
            gpu_finding(path, raw, backend, findings);
        }
    }
    if let Some(options) = value.get("runArgs") {
        if stringify_json(options).contains("host.docker.internal") {
            host_gateway_finding(path, raw, backend, findings);
        }
    }
    if stringify_json(value).contains("host.docker.internal") {
        host_gateway_finding(path, raw, backend, findings);
    }
    if let Some(platform) = value
        .get("containerEnv")
        .and_then(|env| env.get("DOCKER_DEFAULT_PLATFORM"))
        .and_then(JsonValue::as_str)
    {
        platform_finding(path, raw, platform, backend, findings);
    }
    if let Some(secrets) = value.get("secrets") {
        let count = secrets.as_object().map_or(1, |items| items.len());
        secrets_finding(path, raw, count, backend, findings);
    }
}

fn inspect_compose(
    path: &Path,
    raw: &str,
    value: &YamlValue,
    backend: Backend,
    findings: &mut Vec<Finding>,
) {
    if backend == Backend::AppleContainer {
        push_finding(
            findings,
            "APPLE-COMPOSE-UNSUPPORTED",
            Severity::Error,
            "Compose project cannot run directly",
            path,
            Some(1),
            "A Compose configuration is present.",
            "Apple Container does not implement the Docker Compose API used by this project.",
            "Keep a Docker-compatible backend or replace Compose orchestration before switching.",
        );
    }
    let Some(services) = yaml_get(value, "services").and_then(YamlValue::as_mapping) else {
        return;
    };
    for (_service_name, service) in services {
        if yaml_get(service, "privileged").and_then(YamlValue::as_bool) == Some(true) {
            privileged_finding(path, raw, backend, findings);
        }
        if let Some(cap_add) = yaml_get(service, "cap_add").and_then(YamlValue::as_sequence) {
            if !cap_add.is_empty() && backend != Backend::DockerDesktop {
                push_finding(
                    findings,
                    "SEC-CAP-ADD",
                    Severity::Warning,
                    "Added Linux capabilities need review",
                    path,
                    line_for(raw, "cap_add:"),
                    &format!("A service adds {} Linux capability entries.", cap_add.len()),
                    "Capability availability differs across VM-backed and rootless runtimes.",
                    "Remove unnecessary capabilities and test the remaining names on the selected backend.",
                );
            }
        }
        if yaml_get(service, "network_mode").and_then(YamlValue::as_str) == Some("host") {
            host_network_finding(path, raw, backend, findings);
        }
        if let Some(platform) = yaml_get(service, "platform").and_then(YamlValue::as_str) {
            platform_finding(path, raw, platform, backend, findings);
        }
        if let Some(volumes) = yaml_get(service, "volumes").and_then(YamlValue::as_sequence) {
            for volume in volumes {
                if let Some(mount) = yaml_mount_string(volume) {
                    inspect_mount(path, raw, &mount, backend, findings);
                }
            }
        }
        let serialized = serde_yaml_ng::to_string(service).unwrap_or_default();
        if serialized.contains("host.docker.internal") {
            host_gateway_finding(path, raw, backend, findings);
        }
        if yaml_get(service, "gpus").is_some()
            || yaml_get(service, "runtime").and_then(YamlValue::as_str) == Some("nvidia")
            || serialized.contains("capabilities: [gpu]")
            || serialized.contains("capabilities:\n") && serialized.contains("- gpu")
        {
            gpu_finding(path, raw, backend, findings);
        }
        if let Some(secrets) = yaml_get(service, "secrets") {
            let count = secrets.as_sequence().map_or(1, |items| items.len());
            secrets_finding(path, raw, count, backend, findings);
        }
    }
    if let Some(secrets) = yaml_get(value, "secrets").and_then(YamlValue::as_mapping) {
        for (name, definition) in secrets {
            let secret_name = name.as_str().unwrap_or("unnamed");
            if let Some(file) = yaml_get(definition, "file").and_then(YamlValue::as_str) {
                let secret_path = path.parent().unwrap_or_else(|| Path::new(".")).join(file);
                if !secret_path.exists() {
                    push_finding(
                        findings,
                        "SECRET-FILE-MISSING",
                        Severity::Error,
                        "Secret file is missing",
                        path,
                        line_for(raw, secret_name),
                        &format!(
                            "Secret '{secret_name}' references a file that is not present; its contents were not read."
                        ),
                        "The first launch will fail when Compose resolves this secret.",
                        "Create the secret file through your team's secure setup process before launching.",
                    );
                }
            }
        }
    }
}

fn inspect_mount(
    path: &Path,
    raw: &str,
    mount: &str,
    backend: Backend,
    findings: &mut Vec<Finding>,
) {
    let lower = mount.to_ascii_lowercase();
    if lower.contains("docker.sock") {
        let (severity, message, remediation) = match backend {
            Backend::DockerDesktop | Backend::Orbstack => (
                Severity::Info,
                "The selected backend exposes a Docker-compatible socket, but mounting it grants control of the host daemon.",
                "Prefer a scoped socket proxy when the workload does not need full daemon access.",
            ),
            Backend::Podman => (
                Severity::Warning,
                "Podman's socket path and Docker API coverage differ from /var/run/docker.sock.",
                "Enable the Podman API socket explicitly and mount its actual path, or remove the daemon dependency.",
            ),
            Backend::AppleContainer => (
                Severity::Error,
                "Apple Container does not expose a Docker-compatible /var/run/docker.sock.",
                "Remove the Docker socket dependency or keep a Docker-compatible backend.",
            ),
        };
        push_finding(
            findings,
            "MOUNT-DOCKER-SOCKET",
            severity,
            "Docker socket mount is backend-specific",
            path,
            line_for(raw, "docker.sock"),
            "A mount targets the Docker daemon socket; no socket contents were accessed.",
            message,
            remediation,
        );
    }
    if lower.contains("consistency=") || lower.ends_with(":cached") || lower.ends_with(":delegated")
    {
        let severity = if backend == Backend::DockerDesktop {
            Severity::Info
        } else {
            Severity::Warning
        };
        push_finding(
            findings,
            "MOUNT-CONSISTENCY",
            severity,
            "Mount consistency option is not portable",
            path,
            line_for(raw, "consistency"),
            "A bind mount requests cached, delegated, or an explicit consistency mode.",
            "Consistency hints are Docker Desktop-specific or ignored by the selected backend.",
            "Remove the hint unless profiling proves it is required on Docker Desktop.",
        );
    }
    let source = mount_source(mount);
    if let Some(source) = source {
        let is_absolute = source.starts_with('/') && !source.starts_with("/var/run/docker.sock");
        if is_absolute && !source.starts_with("/Users/") && !source.starts_with("/tmp/") {
            let severity = match backend {
                Backend::AppleContainer => Severity::Error,
                Backend::Podman => Severity::Warning,
                _ => Severity::Info,
            };
            push_finding(
                findings,
                "MOUNT-HOST-PATH",
                severity,
                "Absolute host path may not be shared",
                path,
                line_for(raw, source),
                "A bind mount uses an absolute host path outside /Users and /tmp.",
                "Mac backends expose different host directories to their Linux virtual machines.",
                "Use a path relative to the project, a named volume, or document the required file-sharing setup.",
            );
        }
    }
}

fn privileged_finding(path: &Path, raw: &str, backend: Backend, findings: &mut Vec<Finding>) {
    let (severity, message, remediation) = match backend {
        Backend::DockerDesktop | Backend::Orbstack => (
            Severity::Warning,
            "Privileged mode is available inside the backend VM but broadens container access.",
            "Replace privileged mode with the minimum required capabilities and devices.",
        ),
        Backend::Podman => (
            Severity::Warning,
            "Rootless Podman cannot provide the same privileged environment as a Docker daemon.",
            "Test under a rootful Podman machine or replace privileged mode with explicit capabilities.",
        ),
        Backend::AppleContainer => (
            Severity::Error,
            "Apple Container does not provide Docker-equivalent privileged mode.",
            "Remove privileged mode or retain a backend that supports the required kernel access.",
        ),
    };
    push_finding(
        findings,
        "SEC-PRIVILEGED",
        severity,
        "Privileged mode reduces portability",
        path,
        line_for(raw, "privileged"),
        "A container or service requests privileged mode.",
        message,
        remediation,
    );
}

fn host_network_finding(path: &Path, raw: &str, backend: Backend, findings: &mut Vec<Finding>) {
    let (severity, message, remediation) = match backend {
        Backend::DockerDesktop => (
            Severity::Warning,
            "Host networking requires Docker Desktop 4.34+ and must be enabled in Settings.",
            "Verify the installed Docker Desktop version and enable host networking, or publish explicit ports.",
        ),
        Backend::Podman => (
            Severity::Warning,
            "Host mode targets the Podman machine, not the macOS network namespace.",
            "Publish explicit ports and address host services through host.containers.internal.",
        ),
        Backend::Orbstack => (
            Severity::Warning,
            "Host mode is VM-scoped and can differ from Docker Desktop behavior.",
            "Publish explicit ports and test host reachability before switching.",
        ),
        Backend::AppleContainer => (
            Severity::Error,
            "Docker-style host networking is unavailable.",
            "Replace host network mode with explicit published ports.",
        ),
    };
    push_finding(
        findings,
        "NET-HOST-MODE",
        severity,
        "Host network mode is not portable on macOS",
        path,
        line_for(raw, "network_mode").or_else(|| line_for(raw, "network=host")),
        "A container requests host network mode.",
        message,
        remediation,
    );
}

fn host_gateway_finding(path: &Path, raw: &str, backend: Backend, findings: &mut Vec<Finding>) {
    let (severity, message, remediation) = match backend {
        Backend::DockerDesktop | Backend::Orbstack => (
            Severity::Info,
            "The selected backend provides the Docker host gateway alias.",
            "Keep the hostname configurable for Linux and non-Docker environments.",
        ),
        Backend::Podman => (
            Severity::Warning,
            "Podman standardizes on host.containers.internal; Docker's alias may depend on machine configuration.",
            "Use a configurable hostname and default to host.containers.internal on Podman.",
        ),
        Backend::AppleContainer => (
            Severity::Error,
            "The Docker host gateway alias is not provided.",
            "Use published ports and an Apple Container-supported host address.",
        ),
    };
    push_finding(
        findings,
        "NET-HOST-GATEWAY",
        severity,
        "Host gateway hostname is backend-specific",
        path,
        line_for(raw, "host.docker.internal"),
        "The configuration refers to host.docker.internal.",
        message,
        remediation,
    );
}

fn platform_finding(
    path: &Path,
    raw: &str,
    platform: &str,
    backend: Backend,
    findings: &mut Vec<Finding>,
) {
    let requested = if platform.contains("amd64") || platform.contains("x86_64") {
        "amd64"
    } else if platform.contains("arm64") || platform.contains("aarch64") {
        "arm64"
    } else {
        "unknown"
    };
    let host = normalize_arch(std::env::consts::ARCH);
    if requested != "unknown" && requested != host {
        let (severity, message, remediation) = match backend {
            Backend::AppleContainer => (
                Severity::Error,
                "Apple Container does not transparently provide Docker Desktop's cross-architecture image execution path.",
                "Build or select an image for the Mac host architecture.",
            ),
            Backend::Podman => (
                Severity::Warning,
                "Cross-architecture execution requires binfmt/QEMU support inside the Podman machine.",
                "Install emulation support or publish a multi-architecture image.",
            ),
            Backend::DockerDesktop | Backend::Orbstack => (
                Severity::Warning,
                "Emulation is available but can be slower and may expose architecture-sensitive failures.",
                "Prefer a multi-architecture image matching the host.",
            ),
        };
        push_finding(
            findings,
            "ARCH-MISMATCH",
            severity,
            "Requested image architecture differs from this host",
            path,
            line_for(raw, "platform"),
            &format!("The project requests {requested}; the current host is {host}."),
            message,
            remediation,
        );
    } else {
        push_finding(
            findings,
            "ARCH-PINNED",
            Severity::Info,
            "Image architecture is pinned",
            path,
            line_for(raw, "platform"),
            &format!("The project explicitly requests platform '{platform}'."),
            "Pinned platforms reduce portability when teammates use different Mac generations.",
            "Prefer a multi-architecture image and remove the pin when possible.",
        );
    }
}

fn gpu_finding(path: &Path, raw: &str, backend: Backend, findings: &mut Vec<Finding>) {
    let (severity, message) = match backend {
        Backend::DockerDesktop => (
            Severity::Error,
            "NVIDIA GPU passthrough is not available from Docker Desktop on macOS.",
        ),
        Backend::Podman => (
            Severity::Error,
            "NVIDIA GPU passthrough is not available from a Podman machine on macOS.",
        ),
        Backend::Orbstack => (
            Severity::Error,
            "NVIDIA GPU passthrough is not available from OrbStack on macOS.",
        ),
        Backend::AppleContainer => (
            Severity::Error,
            "Docker Compose GPU device requests are not supported by Apple Container.",
        ),
    };
    push_finding(
        findings,
        "GPU-NVIDIA-MAC",
        severity,
        "GPU request cannot be satisfied on this Mac backend",
        path,
        line_for(raw, "gpus").or_else(|| line_for(raw, "nvidia")),
        "A service requests an NVIDIA/GPU runtime or GPU device capability.",
        message,
        "Provide a CPU fallback for local development or run the GPU workload on a supported Linux host.",
    );
}

fn secrets_finding(
    path: &Path,
    raw: &str,
    count: usize,
    backend: Backend,
    findings: &mut Vec<Finding>,
) {
    let severity = if backend == Backend::AppleContainer {
        Severity::Warning
    } else {
        Severity::Info
    };
    let message = if backend == Backend::AppleContainer {
        "The selected backend does not implement Compose secret mounting."
    } else {
        "Secret declarations are portable only when their source files or external stores exist locally."
    };
    push_finding(
        findings,
        "SECRET-DECLARED",
        severity,
        "Secret inputs require local setup",
        path,
        line_for(raw, "secrets"),
        &format!(
            "The configuration declares {count} secret reference(s); secret values were not read or printed."
        ),
        message,
        "Provision secret inputs through the team's secure setup process before the first launch.",
    );
}

fn probe_backend(backend: Backend) -> Result<ProbeResult, DoctorError> {
    let (program, args, command_label): (&str, &[&str], &str) = match backend {
        Backend::DockerDesktop => (
            "docker",
            &[
                "version",
                "--format",
                "{{.Server.Version}}|{{.Server.Os}}|{{.Server.Arch}}",
            ],
            "docker version",
        ),
        Backend::Podman => (
            "podman",
            &[
                "version",
                "--format",
                "{{.Server.Version}}|{{.Server.Os}}|{{.Server.Arch}}",
            ],
            "podman version",
        ),
        Backend::Orbstack => ("orbctl", &["version"], "orbctl version"),
        Backend::AppleContainer => (
            "container",
            &["system", "status"],
            "container system status",
        ),
    };
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            DoctorError::Probe(format!(
                "could not run read-only probe '{command_label}': {error}"
            ))
        })?;
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(DoctorError::Probe(format!(
                    "read-only probe '{command_label}' timed out after 4 seconds"
                )));
            }
            Err(error) => return Err(DoctorError::Probe(format!("probe failed: {error}"))),
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|error| DoctorError::Probe(format!("could not collect probe result: {error}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        return Err(DoctorError::Probe(format!(
            "read-only probe '{command_label}' found the executable, but the selected backend did not respond successfully"
        )));
    }
    let normalized = normalize_probe_output(&stdout);
    let parts: Vec<&str> = normalized.split('|').collect();
    let version = parts.first().and_then(|part| extract_version(part));
    let architecture = parts
        .get(2)
        .map(|part| normalize_arch(part).to_owned())
        .filter(|part| !part.is_empty());
    Ok(ProbeResult {
        command: command_label.to_owned(),
        available: true,
        healthy: true,
        version,
        architecture,
        note: "Backend responded to a read-only status command.".to_owned(),
    })
}

fn apply_version_rules(backend: Backend, probe: &ProbeResult, findings: &mut [Finding]) {
    if backend != Backend::DockerDesktop {
        return;
    }
    let Some(version) = probe.version.as_deref() else {
        return;
    };
    if version_at_least(version, 4, 34) {
        return;
    }
    for finding in findings
        .iter_mut()
        .filter(|finding| finding.rule_id == "NET-HOST-MODE")
    {
        finding.severity = Severity::Error;
        finding.title = "Docker Desktop version is too old for host networking".to_owned();
        finding.message = format!(
            "The probed Docker Desktop version ({version}) predates host networking support in 4.34."
        );
    }
}

fn version_at_least(value: &str, wanted_major: u64, wanted_minor: u64) -> bool {
    let mut parts = value
        .trim_start_matches(|character: char| !character.is_ascii_digit())
        .split('.')
        .filter_map(|part| {
            let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
            (!digits.is_empty())
                .then(|| digits.parse::<u64>().ok())
                .flatten()
        });
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    (major, minor) >= (wanted_major, wanted_minor)
}

fn normalize_probe_output(value: &str) -> String {
    value
        .lines()
        .take(3)
        .map(|line| {
            line.chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || ".|-_ /".contains(*ch))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(160)
        .collect()
}

fn extract_version(value: &str) -> Option<String> {
    value.split_whitespace().find_map(|token| {
        let trimmed = token.trim_matches(|ch: char| {
            !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-' && ch != '_'
        });
        trimmed
            .chars()
            .any(|ch| ch.is_ascii_digit())
            .then(|| trimmed.to_owned())
    })
}

fn yaml_get<'a>(value: &'a YamlValue, key: &str) -> Option<&'a YamlValue> {
    value.as_mapping()?.get(YamlValue::String(key.to_owned()))
}

fn yaml_mount_string(value: &YamlValue) -> Option<String> {
    if let Some(value) = value.as_str() {
        return Some(value.to_owned());
    }
    let source = yaml_get(value, "source").and_then(YamlValue::as_str)?;
    let target = yaml_get(value, "target")
        .and_then(YamlValue::as_str)
        .unwrap_or("");
    let consistency = yaml_get(value, "consistency").and_then(YamlValue::as_str);
    Some(match consistency {
        Some(consistency) => format!("{source}:{target}:consistency={consistency}"),
        None => format!("{source}:{target}"),
    })
}

fn mount_source(mount: &str) -> Option<&str> {
    if mount.contains("source=") {
        mount
            .split(',')
            .find_map(|item| item.trim().strip_prefix("source="))
    } else {
        mount.split(':').next()
    }
}

fn stringify_json(value: &JsonValue) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn line_for(raw: &str, needle: &str) -> Option<usize> {
    raw.lines()
        .position(|line| line.contains(needle))
        .map(|line| line + 1)
}

fn normalize_arch(value: &str) -> &str {
    match value.trim().to_ascii_lowercase().as_str() {
        "x86_64" | "x86-64" | "amd64" => "amd64",
        "aarch64" | "arm64" => "arm64",
        _ => value.trim(),
    }
}

fn display_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[allow(clippy::too_many_arguments)]
fn push_finding(
    findings: &mut Vec<Finding>,
    rule_id: &str,
    severity: Severity,
    title: &str,
    path: &Path,
    line: Option<usize>,
    evidence: &str,
    message: &str,
    remediation: &str,
) {
    findings.push(Finding {
        rule_id: rule_id.to_owned(),
        severity,
        title: title.to_owned(),
        source: path.display().to_string(),
        line,
        evidence: evidence.to_owned(),
        message: message.to_owned(),
        remediation: remediation.to_owned(),
    });
}

fn deduplicate(findings: &mut Vec<Finding>) {
    let mut seen = HashSet::new();
    findings.retain(|finding| {
        seen.insert((
            finding.rule_id.clone(),
            finding.source.clone(),
            finding.line,
        ))
    });
}

fn summarize(findings: &[Finding]) -> Summary {
    let mut summary = Summary::default();
    for finding in findings {
        match finding.severity {
            Severity::Error => summary.errors += 1,
            Severity::Warning => summary.warnings += 1,
            Severity::Info => summary.info += 1,
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn documented_json_example_finds_socket_and_privileged_mode() {
        let directory = tempdir().unwrap();
        let devcontainer_dir = directory.path().join(".devcontainer");
        fs::create_dir(&devcontainer_dir).unwrap();
        fs::write(
            devcontainer_dir.join("devcontainer.json"),
            r#"{
          // comments and trailing commas are valid
          "image": "example/dev:latest",
          "privileged": true,
          "mounts": ["source=/var/run/docker.sock,target=/var/run/docker.sock,type=bind"],
        }"#,
        )
        .unwrap();

        let report = inspect(directory.path(), Backend::AppleContainer, false).unwrap();
        assert!(!report.compatible);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.rule_id == "MOUNT-DOCKER-SOCKET")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.rule_id == "SEC-PRIVILEGED")
        );
        assert!(report.probe.is_none());
    }

    #[test]
    fn linked_compose_is_discovered_and_secret_values_are_absent() {
        let directory = tempdir().unwrap();
        let devcontainer_dir = directory.path().join(".devcontainer");
        fs::create_dir(&devcontainer_dir).unwrap();
        fs::write(
            devcontainer_dir.join("devcontainer.json"),
            r#"{"dockerComposeFile":"compose.yml","service":"app"}"#,
        )
        .unwrap();
        fs::write(devcontainer_dir.join("compose.yml"), "services:\n  app:\n    image: local\n    secrets:\n      - token\nsecrets:\n  token:\n    file: ./missing-token.txt\n").unwrap();

        let report = inspect(directory.path(), Backend::Podman, false).unwrap();
        let output = serde_json::to_string(&report).unwrap();
        assert_eq!(report.project_files.len(), 2);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.rule_id == "SECRET-FILE-MISSING")
        );
        assert!(!output.contains("secret-value"));
    }

    #[test]
    fn malformed_config_is_an_input_error() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("compose.yml"), "services: [broken").unwrap();
        assert!(matches!(
            inspect(directory.path(), Backend::Podman, false),
            Err(DoctorError::Parse { .. })
        ));
    }

    #[test]
    fn empty_project_has_a_clear_error() {
        let directory = tempdir().unwrap();
        let error = inspect(directory.path(), Backend::Orbstack, false).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("no devcontainer.json or Compose file")
        );
    }

    #[test]
    fn compose_gpu_and_network_are_reported() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("compose.yaml"), "services:\n  app:\n    image: example\n    network_mode: host\n    gpus: all\n    platform: linux/amd64\n").unwrap();
        let report = inspect(directory.path(), Backend::AppleContainer, false).unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.rule_id == "GPU-NVIDIA-MAC")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.rule_id == "NET-HOST-MODE")
        );
    }

    #[test]
    fn backend_version_comparison_handles_prefixes_and_patches() {
        assert!(version_at_least("v4.34.0", 4, 34));
        assert!(version_at_least("27.1.2", 4, 34));
        assert!(!version_at_least("4.33.9", 4, 34));
    }
}
