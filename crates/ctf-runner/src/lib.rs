//! External process, Python worker, and operation runner assembly lives here.

use ctf_core::{OperationRegistry, OperationRunner, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

pub fn default_runner() -> Result<OperationRunner> {
    let registry = OperationRegistry::load_default()?;
    let mut runner = OperationRunner::new(registry);
    ctf_codecs::register_handlers(&mut runner);
    ctf_crypto::register_handlers(&mut runner);
    ctf_pwn::register_handlers(&mut runner);
    ctf_stego::register_handlers(&mut runner);
    ctf_web::register_handlers(&mut runner);
    Ok(runner)
}

#[derive(Debug, Clone)]
pub struct PythonWorkerConfig {
    pub python: String,
    pub module: String,
    pub cwd: PathBuf,
}

impl Default for PythonWorkerConfig {
    fn default() -> Self {
        Self {
            python: "python3".to_string(),
            module: "ctf_toolbox_worker".to_string(),
            cwd: PathBuf::from("python"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerRequest {
    pub id: String,
    pub operation: String,
    #[serde(default)]
    pub input: serde_json::Value,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub limits: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerResponse {
    pub id: Option<String>,
    pub status: String,
    #[serde(default)]
    pub outputs: Vec<serde_json::Value>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("failed to start python worker: {0}")]
    Start(std::io::Error),
    #[error("failed to write python worker request: {0}")]
    Write(std::io::Error),
    #[error("python worker timed out after {0} ms")]
    Timeout(u64),
    #[error("failed to read python worker output: {0}")]
    Output(std::io::Error),
    #[error("failed to serialize python worker request: {0}")]
    Serialize(serde_json::Error),
    #[error("failed to parse python worker response: {0}")]
    Deserialize(serde_json::Error),
    #[error("plugin manifest parse failed: {0}")]
    ManifestToml(toml::de::Error),
    #[error("plugin manifest missing required field: {0}")]
    ManifestMissing(&'static str),
}

pub type RunnerResult<T> = std::result::Result<T, RunnerError>;

pub fn run_python_worker_once(
    config: &PythonWorkerConfig,
    request: &WorkerRequest,
    timeout_ms: u64,
) -> RunnerResult<WorkerResponse> {
    let mut child = Command::new(&config.python)
        .arg("-m")
        .arg(&config.module)
        .current_dir(&config.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(RunnerError::Start)?;

    let mut stdin = child.stdin.take().ok_or_else(|| {
        RunnerError::Start(std::io::Error::other("python worker stdin unavailable"))
    })?;
    let line = serde_json::to_string(request).map_err(RunnerError::Serialize)?;
    stdin
        .write_all(format!("{line}\n").as_bytes())
        .map_err(RunnerError::Write)?;
    drop(stdin);

    if child
        .wait_timeout(Duration::from_millis(timeout_ms))
        .map_err(RunnerError::Output)?
        .is_none()
    {
        let _ = child.kill();
        return Err(RunnerError::Timeout(timeout_ms));
    }

    let output = child.wait_with_output().map_err(RunnerError::Output)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or("{}");
    serde_json::from_str(first_line).map_err(RunnerError::Deserialize)
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: String,
    #[serde(default)]
    pub description: String,
    pub permissions: PluginPermissions,
    #[serde(default)]
    pub operations: Vec<PluginOperation>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PluginPermissions {
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub read_files: bool,
    #[serde(default)]
    pub write_files: bool,
    #[serde(default)]
    pub execute_process: bool,
    #[serde(default)]
    pub secrets: bool,
    #[serde(default)]
    pub dangerous_templates: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PluginOperation {
    pub id: String,
    pub name_zh: String,
    pub name_en: String,
    #[serde(default)]
    pub input: Vec<String>,
    #[serde(default)]
    pub output: Vec<String>,
    pub safety: String,
}

pub fn validate_plugin_manifest(text: &str) -> RunnerResult<PluginManifest> {
    let manifest: PluginManifest = toml::from_str(text).map_err(RunnerError::ManifestToml)?;
    if manifest.id.trim().is_empty() {
        return Err(RunnerError::ManifestMissing("id"));
    }
    if manifest.operations.iter().any(|op| op.id.trim().is_empty()) {
        return Err(RunnerError::ManifestMissing("operations.id"));
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_plugin_manifest() {
        let manifest = validate_plugin_manifest(
            r#"
id = "pcap.scapy"
name = "PCAP Scapy Adapter"
version = "0.1.0"
kind = "python_worker"

[permissions]
read_files = true
write_files = true

[[operations]]
id = "pcap.http.extract"
name_zh = "PCAP HTTP 提取"
name_en = "Extract HTTP Objects"
input = ["file"]
output = ["artifact_dir", "json"]
safety = "local_only"
"#,
        )
        .expect("manifest should validate");
        assert_eq!(manifest.id, "pcap.scapy");
        assert!(manifest.permissions.read_files);
    }
}
