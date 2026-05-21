use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct OperationSpec {
    pub id: String,
    pub name_zh: String,
    pub name_en: String,
    pub category: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub input: Vec<String>,
    #[serde(default)]
    pub output: Vec<String>,
    pub backend: String,
    pub safety: String,
    #[serde(default)]
    pub deterministic: bool,
    #[serde(default)]
    pub batchable: bool,
    pub priority: String,
    #[serde(default)]
    pub secrets: Option<SecretPolicy>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SecretPolicy {
    #[serde(default)]
    pub redact_by_default: bool,
    #[serde(default)]
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryFile {
    pub schema_version: u32,
    #[serde(default)]
    pub operations: Vec<OperationSpec>,
}

#[derive(Debug, Clone)]
pub struct OperationRegistry {
    schema_version: u32,
    operations: Vec<OperationSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationInput {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationOutput {
    pub kind: String,
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskLimits {
    pub timeout_ms: u64,
    pub max_output_bytes: usize,
}

impl Default for TaskLimits {
    fn default() -> Self {
        Self {
            timeout_ms: 5_000,
            max_output_bytes: 10 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationRequest {
    pub operation: String,
    pub input: OperationInput,
    #[serde(default)]
    pub limits: TaskLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationResponse {
    pub status: String,
    #[serde(default)]
    pub outputs: Vec<OperationOutput>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorReport {
    pub code: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CtfError {
    #[error("failed to read registry at {path}: {source}")]
    RegistryRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse registry at {path}: {source}")]
    RegistryParse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("operation not found: {0}")]
    OperationNotFound(String),
    #[error("operation is registered but not implemented yet: {0}")]
    UnsupportedOperation(String),
    #[error("input kind `{kind}` is not accepted by operation `{operation}`")]
    InvalidInputKind { operation: String, kind: String },
    #[error("failed to read input file at {path}: {source}")]
    InputRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("input is not valid UTF-8")]
    InvalidUtf8,
    #[error("{0}")]
    InvalidInput(String),
    #[error("operation output exceeds max_output_bytes ({limit})")]
    OutputLimitExceeded { limit: usize },
}

pub type Result<T> = std::result::Result<T, CtfError>;

impl OperationRegistry {
    pub fn load_default() -> Result<Self> {
        let path = default_registry_path();
        Self::load_file(path)
    }

    pub fn load_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let text = fs::read_to_string(&path).map_err(|source| CtfError::RegistryRead {
            path: path.clone(),
            source,
        })?;
        let file: RegistryFile =
            toml::from_str(&text).map_err(|source| CtfError::RegistryParse {
                path: path.clone(),
                source,
            })?;
        Ok(Self {
            schema_version: file.schema_version,
            operations: file.operations,
        })
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn operations(&self) -> &[OperationSpec] {
        &self.operations
    }

    pub fn find(&self, id: &str) -> Option<&OperationSpec> {
        self.operations.iter().find(|op| op.id == id)
    }

    pub fn search(&self, query: &str) -> Vec<&OperationSpec> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return self.operations.iter().collect();
        }

        self.operations
            .iter()
            .filter(|op| {
                op.id.to_lowercase().contains(&query)
                    || op.name_zh.to_lowercase().contains(&query)
                    || op.name_en.to_lowercase().contains(&query)
                    || op.category.to_lowercase().contains(&query)
                    || op
                        .aliases
                        .iter()
                        .any(|alias| alias.to_lowercase().contains(&query))
            })
            .collect()
    }
}

pub struct OperationRunner {
    registry: OperationRegistry,
    handlers: HashMap<String, OperationHandler>,
}

pub type OperationHandler = fn(&OperationSpec, &OperationRequest) -> Result<OperationResponse>;

impl OperationRunner {
    pub fn new(registry: OperationRegistry) -> Self {
        Self {
            registry,
            handlers: HashMap::new(),
        }
    }

    pub fn registry(&self) -> &OperationRegistry {
        &self.registry
    }

    pub fn register_handler(&mut self, operation: impl Into<String>, handler: OperationHandler) {
        self.handlers.insert(operation.into(), handler);
    }

    pub fn run(&self, request: OperationRequest) -> Result<OperationResponse> {
        let spec = self
            .registry
            .find(&request.operation)
            .ok_or_else(|| CtfError::OperationNotFound(request.operation.clone()))?;

        if !spec.input.iter().any(|kind| kind == &request.input.kind) {
            return Err(CtfError::InvalidInputKind {
                operation: spec.id.clone(),
                kind: request.input.kind,
            });
        }

        let Some(handler) = self.handlers.get(&spec.id) else {
            return Err(CtfError::UnsupportedOperation(spec.id.clone()));
        };

        let response = handler(spec, &request)?;
        enforce_output_limit(&response, request.limits.max_output_bytes)?;
        Ok(response)
    }
}

impl OperationRequest {
    pub fn input_bytes(&self) -> Result<Vec<u8>> {
        match self.input.kind.as_str() {
            "text" | "bytes" => Ok(self.input.value.as_bytes().to_vec()),
            "file" => {
                let path = PathBuf::from(&self.input.value);
                fs::read(&path).map_err(|source| CtfError::InputRead { path, source })
            }
            _ => Err(CtfError::InvalidInputKind {
                operation: self.operation.clone(),
                kind: self.input.kind.clone(),
            }),
        }
    }

    pub fn input_text(&self) -> Result<String> {
        match self.input.kind.as_str() {
            "text" => Ok(self.input.value.clone()),
            "bytes" => String::from_utf8(self.input.value.as_bytes().to_vec())
                .map_err(|_| CtfError::InvalidUtf8),
            "file" => {
                let path = PathBuf::from(&self.input.value);
                fs::read_to_string(&path).map_err(|source| CtfError::InputRead { path, source })
            }
            _ => Err(CtfError::InvalidInputKind {
                operation: self.operation.clone(),
                kind: self.input.kind.clone(),
            }),
        }
    }
}

pub fn error_report(error: &CtfError) -> ErrorReport {
    match error {
        CtfError::RegistryRead { .. } => ErrorReport {
            code: "registry_read".to_string(),
            message: error.to_string(),
        },
        CtfError::RegistryParse { .. } => ErrorReport {
            code: "registry_parse".to_string(),
            message: error.to_string(),
        },
        CtfError::OperationNotFound(_) => ErrorReport {
            code: "operation_not_found".to_string(),
            message: error.to_string(),
        },
        CtfError::UnsupportedOperation(_) => ErrorReport {
            code: "unsupported_operation".to_string(),
            message: error.to_string(),
        },
        CtfError::InvalidInputKind { .. } => ErrorReport {
            code: "invalid_input_kind".to_string(),
            message: error.to_string(),
        },
        CtfError::InputRead { .. } => ErrorReport {
            code: "input_read".to_string(),
            message: error.to_string(),
        },
        CtfError::InvalidUtf8 => ErrorReport {
            code: "invalid_utf8".to_string(),
            message: error.to_string(),
        },
        CtfError::InvalidInput(_) => ErrorReport {
            code: "invalid_input".to_string(),
            message: error.to_string(),
        },
        CtfError::OutputLimitExceeded { .. } => ErrorReport {
            code: "output_limit_exceeded".to_string(),
            message: error.to_string(),
        },
    }
}

fn enforce_output_limit(response: &OperationResponse, limit: usize) -> Result<()> {
    let total: usize = response
        .outputs
        .iter()
        .map(|output| output.value.len())
        .sum();
    if total > limit {
        return Err(CtfError::OutputLimitExceeded { limit });
    }
    Ok(())
}

fn default_registry_path() -> PathBuf {
    if let Ok(path) = env::var("CTF_TOOLS_REGISTRY") {
        return PathBuf::from(path);
    }

    let cwd_path = PathBuf::from("registry/operations.toml");
    if cwd_path.exists() {
        return cwd_path;
    }

    if let Ok(exe_path) = env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        let executable_relative_path = exe_dir.join("registry/operations.toml");
        if executable_relative_path.exists() {
            return executable_relative_path;
        }

        if let Some(contents_dir) = exe_dir.parent() {
            let macos_app_resource_path = contents_dir.join("Resources/registry/operations.toml");
            if macos_app_resource_path.exists() {
                return macos_app_resource_path;
            }
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../registry/operations.toml")
        .components()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_default_registry() {
        let registry = OperationRegistry::load_default().expect("registry should load");
        assert_eq!(registry.schema_version(), 1);
        assert!(registry.find("base64.decode").is_some());
    }

    #[test]
    fn search_matches_aliases() {
        let registry = OperationRegistry::load_default().expect("registry should load");
        let matches = registry.search("请求包转python");
        assert!(
            matches
                .iter()
                .any(|op| op.id == "http.raw.to_python_requests")
        );
    }

    #[test]
    fn runner_executes_registered_handler() {
        fn handler(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
            Ok(OperationResponse {
                status: "ok".to_string(),
                outputs: vec![OperationOutput {
                    kind: "text".to_string(),
                    label: "echo".to_string(),
                    value: request.input.value.clone(),
                }],
                warnings: vec![],
            })
        }

        let registry = OperationRegistry::load_default().expect("registry should load");
        let mut runner = OperationRunner::new(registry);
        runner.register_handler("base64.decode", handler);
        let response = runner
            .run(OperationRequest {
                operation: "base64.decode".to_string(),
                input: OperationInput {
                    kind: "text".to_string(),
                    value: "abc".to_string(),
                },
                limits: TaskLimits::default(),
            })
            .expect("handler should run");
        assert_eq!(response.outputs[0].value, "abc");
    }

    #[test]
    fn runner_rejects_invalid_input_kind() {
        let registry = OperationRegistry::load_default().expect("registry should load");
        let runner = OperationRunner::new(registry);
        let error = runner
            .run(OperationRequest {
                operation: "base64.decode".to_string(),
                input: OperationInput {
                    kind: "json".to_string(),
                    value: "{}".to_string(),
                },
                limits: TaskLimits::default(),
            })
            .expect_err("invalid kind should fail");
        assert_eq!(error_report(&error).code, "invalid_input_kind");
    }
}
