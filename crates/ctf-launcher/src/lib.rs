use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const ALL_ID: &str = "__all__";
pub const FAVORITES_ID: &str = "__fav__";
pub const RECENT_ID: &str = "__recent__";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    Python,
    Java,
    Shell,
    Gui,
    Url,
}

impl ToolType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Java => "java",
            Self::Shell => "shell",
            Self::Gui => "gui",
            Self::Url => "url",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentType {
    Python,
    Venv,
    Conda,
    Java,
}

impl EnvironmentType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Venv => "venv",
            Self::Conda => "conda",
            Self::Java => "java",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub env_type: EnvironmentType,
    pub path: String,
    #[serde(default)]
    pub version: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub javafx: bool,
}

impl Environment {
    pub fn is_user_defined(&self) -> bool {
        self.source == "user"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherTool {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub tool_type: ToolType,
    pub path: String,
    #[serde(default)]
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_id: Option<String>,
    #[serde(default)]
    pub args: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub last_used: u64,
}

impl LauncherTool {
    pub fn matches_query(&self, query: &str) -> bool {
        if query.trim().is_empty() {
            return true;
        }
        let query = query.to_lowercase();
        self.name.to_lowercase().contains(&query)
            || self.description.to_lowercase().contains(&query)
            || self.path.to_lowercase().contains(&query)
            || self
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&query))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub order: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvironmentDefaults {
    #[serde(default)]
    pub python: String,
    #[serde(default)]
    pub java: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvironmentRegistry {
    #[serde(default)]
    pub environments: Vec<Environment>,
    #[serde(default)]
    pub defaults: EnvironmentDefaults,
}

impl EnvironmentRegistry {
    pub fn find(&self, env_id: &str) -> Option<&Environment> {
        self.environments.iter().find(|env| env.id == env_id)
    }

    pub fn default_python(&self) -> Option<&Environment> {
        self.find(&self.defaults.python)
    }

    pub fn default_java(&self) -> Option<&Environment> {
        self.find(&self.defaults.java)
    }

    pub fn merge_scanned(&mut self, scanned: Vec<Environment>) {
        let mut seen = HashSet::new();
        let mut merged = Vec::new();
        for env in scanned {
            if seen.insert(env.id.clone()) {
                merged.push(env);
            }
        }
        for env in self.environments.iter().filter(|env| env.is_user_defined()) {
            if seen.insert(env.id.clone()) {
                merged.push(env.clone());
            }
        }
        self.environments = merged;
        if !self.defaults.python.is_empty() && self.find(&self.defaults.python).is_none() {
            self.defaults.python.clear();
        }
        if !self.defaults.java.is_empty() && self.find(&self.defaults.java).is_none() {
            self.defaults.java.clear();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherSettings {
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LauncherStore {
    data_dir: PathBuf,
}

impl LauncherStore {
    pub fn default_data_dir() -> PathBuf {
        home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("Application Support")
            .join("CTF Tools")
            .join("launcher")
    }

    pub fn new() -> Self {
        Self {
            data_dir: Self::default_data_dir(),
        }
    }

    pub fn from_dir(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn ensure_data_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.data_dir)
            .with_context(|| format!("create launcher data dir {}", self.data_dir.display()))
    }

    pub fn load_tools(&self) -> Vec<LauncherTool> {
        read_json(&self.data_dir.join("tools.json"), Vec::new())
    }

    pub fn save_tools(&self, tools: &[LauncherTool]) -> Result<()> {
        self.write_json("tools.json", tools)
    }

    pub fn load_categories(&self) -> Vec<Category> {
        read_json(&self.data_dir.join("categories.json"), Vec::new())
    }

    pub fn save_categories(&self, categories: &[Category]) -> Result<()> {
        self.write_json("categories.json", categories)
    }

    pub fn load_environments(&self) -> EnvironmentRegistry {
        read_json(
            &self.data_dir.join("environments.json"),
            EnvironmentRegistry::default(),
        )
    }

    pub fn save_environments(&self, registry: &EnvironmentRegistry) -> Result<()> {
        self.write_json("environments.json", registry)
    }

    pub fn load_settings(&self) -> LauncherSettings {
        read_json(
            &self.data_dir.join("settings.json"),
            LauncherSettings::default(),
        )
    }

    pub fn save_settings(&self, settings: &LauncherSettings) -> Result<()> {
        self.write_json("settings.json", settings)
    }

    fn write_json<T: Serialize + ?Sized>(&self, filename: &str, value: &T) -> Result<()> {
        self.ensure_data_dir()?;
        let target = self.data_dir.join(filename);
        let tmp = target.with_extension(format!(
            "{}.tmp",
            target
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("json")
        ));
        let json = serde_json::to_string_pretty(value)?;
        fs::write(&tmp, json).with_context(|| format!("write {}", tmp.display()))?;
        fs::rename(&tmp, &target)
            .with_context(|| format!("replace {} with {}", target.display(), tmp.display()))?;
        Ok(())
    }
}

impl Default for LauncherStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchAction {
    OpenUrl(String),
    OpenPath(String),
    Terminal {
        command: String,
        cwd: Option<String>,
    },
}

pub fn build_launch_action(
    tool: &LauncherTool,
    environments: &EnvironmentRegistry,
) -> Result<LaunchAction> {
    match tool.tool_type {
        ToolType::Url => {
            if tool.path.trim().is_empty() {
                return Err(anyhow!("missing URL"));
            }
            Ok(LaunchAction::OpenUrl(tool.path.clone()))
        }
        ToolType::Gui => {
            if tool.path.trim().is_empty() {
                return Err(anyhow!("missing path"));
            }
            if !Path::new(&tool.path).exists() && !tool.path.starts_with("http") {
                return Err(anyhow!("path does not exist: {}", tool.path));
            }
            Ok(LaunchAction::OpenPath(tool.path.clone()))
        }
        ToolType::Python => {
            let env = tool
                .env_id
                .as_deref()
                .and_then(|id| environments.find(id))
                .or_else(|| environments.default_python())
                .ok_or_else(|| anyhow!("no Python environment configured"))?;
            let python = python_bin(env);
            if !python.exists() {
                return Err(anyhow!("Python does not exist: {}", python.display()));
            }
            let mut command = format!("{} {}", shell_quote(python), shell_quote(&tool.path));
            if !tool.args.trim().is_empty() {
                command.push(' ');
                command.push_str(tool.args.trim());
            }
            Ok(LaunchAction::Terminal {
                command,
                cwd: parent_dir(&tool.path),
            })
        }
        ToolType::Java => {
            let env = tool
                .env_id
                .as_deref()
                .and_then(|id| environments.find(id))
                .or_else(|| environments.default_java())
                .ok_or_else(|| anyhow!("no Java environment configured"))?;
            let java = java_bin(env);
            if !java.exists() {
                return Err(anyhow!("java does not exist: {}", java.display()));
            }
            let mut command = format!("{} -jar {}", shell_quote(java), shell_quote(&tool.path));
            if !tool.args.trim().is_empty() {
                command.push(' ');
                command.push_str(tool.args.trim());
            }
            Ok(LaunchAction::Terminal {
                command,
                cwd: parent_dir(&tool.path),
            })
        }
        ToolType::Shell => {
            if tool.path.trim().is_empty() {
                return Err(anyhow!("missing path"));
            }
            let mut command = if tool.path.ends_with(".sh") {
                format!("bash {}", shell_quote(&tool.path))
            } else {
                shell_quote(&tool.path)
            };
            if !tool.args.trim().is_empty() {
                command.push(' ');
                command.push_str(tool.args.trim());
            }
            Ok(LaunchAction::Terminal {
                command,
                cwd: parent_dir(&tool.path),
            })
        }
    }
}

pub fn launch_tool(tool: &LauncherTool, environments: &EnvironmentRegistry) -> Result<String> {
    let action = build_launch_action(tool, environments)?;
    match action {
        LaunchAction::OpenUrl(url) => {
            open_target(&url)?;
            Ok(format!("Opened {url}"))
        }
        LaunchAction::OpenPath(path) => {
            open_target(&path)?;
            Ok(format!("Launched {}", display_name(&path)))
        }
        LaunchAction::Terminal { command, cwd } => {
            run_in_terminal(&command, cwd.as_deref())?;
            Ok("Running in Terminal".to_string())
        }
    }
}

pub fn record_recent(tools: &mut [LauncherTool], tool_id: &str) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    if let Some(tool) = tools.iter_mut().find(|tool| tool.id == tool_id) {
        tool.last_used = now;
    }
}

pub fn filter_tools(tools: &[LauncherTool], category: &str, query: &str) -> Vec<LauncherTool> {
    let mut filtered = tools
        .iter()
        .filter(|tool| match category {
            ALL_ID => true,
            FAVORITES_ID => tool.favorite,
            RECENT_ID => tool.last_used > 0,
            other => tool.category == other,
        })
        .filter(|tool| tool.matches_query(query))
        .cloned()
        .collect::<Vec<_>>();

    if category == RECENT_ID {
        filtered.sort_by(|left, right| right.last_used.cmp(&left.last_used));
    } else {
        filtered.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });
    }
    filtered
}

pub fn category_counts(tools: &[LauncherTool]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    counts.insert(ALL_ID.to_string(), tools.len());
    counts.insert(
        FAVORITES_ID.to_string(),
        tools.iter().filter(|tool| tool.favorite).count(),
    );
    counts.insert(
        RECENT_ID.to_string(),
        tools.iter().filter(|tool| tool.last_used > 0).count(),
    );
    for tool in tools {
        *counts.entry(tool.category.clone()).or_insert(0) += 1;
    }
    counts
}

pub fn scan_all_environments() -> Vec<Environment> {
    let mut found = BTreeMap::new();
    for env in scan_python()
        .into_iter()
        .chain(scan_venvs())
        .chain(scan_conda())
        .chain(scan_java())
    {
        found.entry(env.id.clone()).or_insert(env);
    }
    found.into_values().collect()
}

pub fn scan_python() -> Vec<Environment> {
    let mut candidates = Vec::new();
    for root in [
        PathBuf::from("/usr/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local/bin"),
    ] {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == "python3" || is_python_minor_name(&name) {
                candidates.push(entry.path());
            }
        }
    }

    let mut seen = HashSet::new();
    let mut output = Vec::new();
    for path in candidates {
        let resolved = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !seen.insert(resolved) {
            continue;
        }
        let version = probe_python(&path);
        if version.is_empty() {
            continue;
        }
        let path_text = path.to_string_lossy();
        let tag = if path_text.contains("/opt/homebrew/") || path_text.contains("/usr/local/") {
            "brew"
        } else if path_text.contains(".local") {
            "user"
        } else {
            "system"
        };
        output.push(Environment {
            id: slug(["py", tag, &version]),
            name: format!("Python {version} ({tag})"),
            env_type: EnvironmentType::Python,
            path: path.display().to_string(),
            version,
            source: "auto".to_string(),
            tags: vec![tag.to_string()],
            javafx: false,
        });
    }
    output
}

pub fn scan_venvs() -> Vec<Environment> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    let mut output = Vec::new();
    for root in [
        home.join(".venvs"),
        home.join("venvs"),
        home.join(".virtualenvs"),
    ] {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let venv = entry.path();
            let python = venv.join("bin/python");
            if !python.exists() {
                continue;
            }
            let version = probe_python(&python);
            let name = entry.file_name().to_string_lossy().to_string();
            output.push(Environment {
                id: slug(["venv", &name]),
                name: format!("{name} (venv)"),
                env_type: EnvironmentType::Venv,
                path: venv.display().to_string(),
                version,
                source: "auto".to_string(),
                tags: vec!["venv".to_string()],
                javafx: false,
            });
        }
    }
    output
}

pub fn scan_conda() -> Vec<Environment> {
    let home = home_dir().unwrap_or_else(|| PathBuf::from("."));
    let roots = [
        PathBuf::from("/opt/homebrew/Caskroom/miniforge/base"),
        home.join("miniforge3"),
        home.join("miniconda3"),
        home.join("anaconda3"),
    ];
    let mut output = Vec::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let base_python = root.join("bin/python");
        if base_python.exists() {
            let version = probe_python(&base_python);
            let root_name = root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("conda");
            output.push(Environment {
                id: slug(["conda", root_name, "base"]),
                name: format!("{root_name} base ({version})"),
                env_type: EnvironmentType::Conda,
                path: root.display().to_string(),
                version,
                source: "auto".to_string(),
                tags: vec!["conda".to_string(), "base".to_string()],
                javafx: false,
            });
        }
        let envs_dir = root.join("envs");
        let Ok(entries) = fs::read_dir(&envs_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let env_path = entry.path();
            let python = env_path.join("bin/python");
            if !python.exists() {
                continue;
            }
            let version = probe_python(&python);
            let name = entry.file_name().to_string_lossy().to_string();
            output.push(Environment {
                id: slug(["conda", &name]),
                name: format!("{name} (conda)"),
                env_type: EnvironmentType::Conda,
                path: env_path.display().to_string(),
                version,
                source: "auto".to_string(),
                tags: vec!["conda".to_string()],
                javafx: false,
            });
        }
    }
    output
}

pub fn scan_java() -> Vec<Environment> {
    let mut output = Vec::new();
    for root in [
        PathBuf::from("/Library/Java/JavaVirtualMachines"),
        PathBuf::from("/opt/homebrew/opt"),
        home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".jdks"),
    ] {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let mut home = path.clone();
            if path.join("Contents/Home").is_dir() {
                home = path.join("Contents/Home");
            } else if path.join("libexec/openjdk.jdk/Contents/Home").is_dir() {
                home = path.join("libexec/openjdk.jdk/Contents/Home");
            } else if root.ends_with("opt")
                && !path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("openjdk"))
            {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("java")
                .to_string();
            if let Some(env) = make_java_env(&home, &name, vec!["jdk".to_string()]) {
                output.push(env);
            }
        }
    }

    if let Some(workspace) = home_dir().map(|home| home.join("Workspace"))
        && workspace.is_dir()
    {
        scan_workspace_jdks(&workspace, &mut output);
    }

    if let Some(java_home) = std::env::var_os("JAVA_HOME").map(PathBuf::from)
        && java_home.is_dir()
        && let Some(env) = make_java_env(
            &java_home,
            "$JAVA_HOME",
            vec!["jdk".to_string(), "env".to_string()],
        )
    {
        output.push(env);
    }
    output
}

fn scan_workspace_jdks(root: &Path, output: &mut Vec<Environment>) {
    let mut stack = vec![root.to_path_buf()];
    let mut seen = HashSet::new();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if path.ends_with("Contents/Home")
                && path.to_string_lossy().contains("/Java_path/Java_")
            {
                let resolved = path.canonicalize().unwrap_or_else(|_| path.clone());
                if seen.insert(resolved) {
                    let hint = path
                        .parent()
                        .and_then(Path::parent)
                        .and_then(Path::file_name)
                        .and_then(|value| value.to_str())
                        .unwrap_or("bundled-java");
                    if let Some(env) =
                        make_java_env(&path, hint, vec!["jdk".to_string(), "bundled".to_string()])
                    {
                        output.push(env);
                    }
                }
            } else if path.components().count() - root.components().count() <= 5 {
                stack.push(path);
            }
        }
    }
}

fn make_java_env(home: &Path, name_hint: &str, mut tags: Vec<String>) -> Option<Environment> {
    let version = probe_java(home);
    if version.is_empty() {
        return None;
    }
    let javafx = has_javafx(home);
    if javafx {
        tags.push("javafx".to_string());
    }
    let fx_label = if javafx { " +FX" } else { "" };
    Some(Environment {
        id: slug(["java", name_hint, &version]),
        name: format!("{name_hint} ({version}){fx_label}"),
        env_type: EnvironmentType::Java,
        path: home.display().to_string(),
        version,
        source: "auto".to_string(),
        tags,
        javafx,
    })
}

fn probe_python(path: &Path) -> String {
    let Ok(output) = Command::new(path).arg("--version").output() else {
        return String::new();
    };
    let text = String::from_utf8_lossy(if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    });
    parse_after_prefix(&text, "Python ")
}

fn probe_java(home: &Path) -> String {
    let java = home.join("bin/java");
    if !java.exists() {
        return String::new();
    }
    let Ok(output) = Command::new(java).arg("-version").output() else {
        return String::new();
    };
    let text = String::from_utf8_lossy(if output.stderr.is_empty() {
        &output.stdout
    } else {
        &output.stderr
    });
    parse_java_version(&text)
}

fn parse_after_prefix(text: &str, prefix: &str) -> String {
    let Some(start) = text.find(prefix).map(|index| index + prefix.len()) else {
        return String::new();
    };
    text[start..]
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn parse_java_version(text: &str) -> String {
    let Some(start) = text
        .find("version \"")
        .map(|index| index + "version \"".len())
    else {
        return String::new();
    };
    text[start..]
        .split('"')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn has_javafx(home: &Path) -> bool {
    for path in [
        home.join("jre/lib/ext/jfxrt.jar"),
        home.join("lib/ext/jfxrt.jar"),
        home.join("jre/lib/jfxrt.jar"),
    ] {
        if path.exists() {
            return true;
        }
    }
    for dir in [home.join("lib"), home.join("jmods")] {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        if entries.flatten().any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .to_lowercase()
                .contains("javafx")
        }) {
            return true;
        }
    }
    false
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, default: T) -> T {
    let Ok(text) = fs::read_to_string(path) else {
        return default;
    };
    serde_json::from_str(&text).unwrap_or(default)
}

fn open_target(target: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(target)
            .spawn()
            .with_context(|| format!("open {target}"))?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .with_context(|| format!("open {target}"))?;
    }
    Ok(())
}

fn run_in_terminal(command: &str, cwd: Option<&str>) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let command = if let Some(cwd) = cwd {
            format!("cd {} && {command}", shell_quote(cwd))
        } else {
            command.to_string()
        };
        let escaped = command.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            "tell application \"Terminal\"\n  activate\n  do script \"{escaped}\"\nend tell"
        );
        Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .context("run command in Terminal")?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let command = if let Some(cwd) = cwd {
            format!("cd {} && {command}", shell_quote(cwd))
        } else {
            command.to_string()
        };
        Command::new("sh")
            .args(["-lc", &command])
            .spawn()
            .context("run command in shell")?;
    }
    Ok(())
}

fn parent_dir(path: &str) -> Option<String> {
    Path::new(path)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.display().to_string())
}

fn python_bin(env: &Environment) -> PathBuf {
    match env.env_type {
        EnvironmentType::Python => PathBuf::from(&env.path),
        EnvironmentType::Venv | EnvironmentType::Conda => {
            PathBuf::from(&env.path).join("bin/python")
        }
        EnvironmentType::Java => PathBuf::from(&env.path),
    }
}

fn java_bin(env: &Environment) -> PathBuf {
    PathBuf::from(&env.path).join("bin/java")
}

fn display_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn shell_quote(value: impl AsRef<Path>) -> String {
    let text = value.as_ref().to_string_lossy();
    if text.is_empty() {
        return "''".to_string();
    }
    if text
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "/._-:+=,".contains(ch))
    {
        return text.to_string();
    }
    format!("'{}'", text.replace('\'', "'\"'\"'"))
}

fn slug<const N: usize>(parts: [&str; N]) -> String {
    let mut output = String::new();
    for part in parts.into_iter().filter(|part| !part.is_empty()) {
        if !output.is_empty() {
            output.push('-');
        }
        for ch in part.chars() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                output.push(ch.to_ascii_lowercase());
            } else if !output.ends_with('-') {
                output.push('-');
            }
        }
    }
    output.trim_matches('-').to_string()
}

fn is_python_minor_name(name: &str) -> bool {
    name.strip_prefix("python3.")
        .is_some_and(|minor| !minor.is_empty() && minor.chars().all(|ch| ch.is_ascii_digit()))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn default_source() -> String {
    "auto".to_string()
}

fn default_theme() -> String {
    "dark".to_string()
}

#[allow(dead_code)]
fn os_string(value: impl Into<OsString>) -> OsString {
    value.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_favorites_recent_and_query() {
        let tools = vec![
            tool("1", "sqlmap", ToolType::Python, "web", true, 10, &["sqli"]),
            tool(
                "2",
                "CyberChef",
                ToolType::Gui,
                "misc",
                false,
                20,
                &["codec"],
            ),
        ];

        assert_eq!(filter_tools(&tools, FAVORITES_ID, "").len(), 1);
        assert_eq!(filter_tools(&tools, RECENT_ID, "")[0].name, "CyberChef");
        assert_eq!(filter_tools(&tools, ALL_ID, "sqli")[0].name, "sqlmap");
    }

    #[test]
    fn builds_shell_launch_action() {
        let tool = LauncherTool {
            id: "x".to_string(),
            name: "script".to_string(),
            tool_type: ToolType::Shell,
            path: "/tmp/run.sh".to_string(),
            category: String::new(),
            env_id: None,
            args: "--flag".to_string(),
            tags: Vec::new(),
            description: String::new(),
            favorite: false,
            last_used: 0,
        };
        let action = build_launch_action(&tool, &EnvironmentRegistry::default()).unwrap();
        assert_eq!(
            action,
            LaunchAction::Terminal {
                command: "bash /tmp/run.sh --flag".to_string(),
                cwd: Some("/tmp".to_string())
            }
        );
    }

    #[test]
    fn stores_json_atomically() {
        let dir = std::env::temp_dir().join(format!(
            "ctf-launcher-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = LauncherStore::from_dir(&dir);
        let tools = vec![tool("1", "demo", ToolType::Url, "links", false, 0, &[])];
        store.save_tools(&tools).unwrap();
        assert_eq!(store.load_tools()[0].name, "demo");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parses_asutools_json_schema() {
        let text = r#"{
          "id": "ab12cd34",
          "name": "sqlmap",
          "type": "python",
          "path": "/path/to/sqlmap.py",
          "args": "--batch",
          "category": "Web",
          "env_id": "venv-sqlmap",
          "tags": ["sqli"],
          "description": "",
          "favorite": true,
          "last_used": 123
        }"#;
        let tool: LauncherTool = serde_json::from_str(text).unwrap();
        assert_eq!(tool.tool_type, ToolType::Python);
        assert_eq!(tool.env_id.as_deref(), Some("venv-sqlmap"));
    }

    fn tool(
        id: &str,
        name: &str,
        tool_type: ToolType,
        category: &str,
        favorite: bool,
        last_used: u64,
        tags: &[&str],
    ) -> LauncherTool {
        LauncherTool {
            id: id.to_string(),
            name: name.to_string(),
            tool_type,
            path: "https://example.com".to_string(),
            category: category.to_string(),
            env_id: None,
            args: String::new(),
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
            description: String::new(),
            favorite,
            last_used,
        }
    }
}
