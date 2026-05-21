use serde::{Deserialize, Serialize};
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
}

impl OperationRunner {
    pub fn new(registry: OperationRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &OperationRegistry {
        &self.registry
    }

    pub fn run(&self, request: OperationRequest) -> Result<OperationResponse> {
        let spec = self
            .registry
            .find(&request.operation)
            .ok_or_else(|| CtfError::OperationNotFound(request.operation.clone()))?;

        Err(CtfError::UnsupportedOperation(spec.id.clone()))
    }
}

fn default_registry_path() -> PathBuf {
    if let Ok(path) = env::var("CTF_TOOLS_REGISTRY") {
        return PathBuf::from(path);
    }

    let cwd_path = PathBuf::from("registry/operations.toml");
    if cwd_path.exists() {
        return cwd_path;
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
}
