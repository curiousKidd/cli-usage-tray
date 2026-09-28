use super::{RootPath, Roots, Tool, ToolRoots};
use std::{collections::BTreeSet, fs, io, path::{Component, Path, PathBuf}};

impl Roots {
    pub fn load() -> Self {
        let path = roots_path();
        fs::read_to_string(path)
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_else(|| Self { custom: Default::default() })
    }

    pub fn save(&self) -> io::Result<()> {
        let path = roots_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_vec_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        fs::write(path, contents)
    }

    pub fn current(&self, tool: Tool) -> Vec<PathBuf> {
        let mut seen = BTreeSet::new();
        defaults(tool).into_iter()
            .chain(self.custom.get(&tool).into_iter().flatten().cloned())
            .map(|path| expand_path(&path))
            .filter_map(|path| {
                let path = normalized(&path);
                seen.insert(path.clone()).then_some(path)
            })
            .collect()
    }

    pub fn to_tool_roots(&self) -> Vec<ToolRoots> {
        [Tool::ClaudeCode, Tool::Codex, Tool::GeminiCli].into_iter().map(|tool| ToolRoots {
            tool,
            defaults: defaults(tool).into_iter().map(|path| root_path(tool, &path)).collect(),
            custom: self.custom.get(&tool).into_iter().flatten()
                .map(|path| root_path(tool, path)).collect(),
        }).collect()
    }

    pub fn set_custom(&mut self, tool: Tool, paths: Vec<String>) {
        self.custom.insert(tool, paths);
    }

    pub fn reset_custom(&mut self) {
        self.custom.clear();
    }
}

pub fn is_log_file(tool: Tool, path: &Path) -> bool {
    matches!((tool, path.extension().and_then(|extension| extension.to_str())),
        (Tool::ClaudeCode | Tool::Codex, Some("jsonl")) |
        (Tool::GeminiCli, Some("json" | "jsonl")))
}

pub fn walk_logs(tool: Tool, root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else { return; };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            walk_logs(tool, &path, files);
        } else if is_log_file(tool, &path) {
            files.push(path);
        }
    }
}

fn defaults(tool: Tool) -> Vec<String> {
    match tool {
        Tool::ClaudeCode => vec![
            "~/.claude/projects".into(), "~/.claude/projects2".into(),
            "~/Library/Application Support/Claude/local-agent-mode-sessions".into(),
        ],
        Tool::Codex => vec![
            "~/.codex/sessions".into(), "~/Library/Application Support/orca/codex-runtime-home".into(),
        ],
        Tool::GeminiCli => vec!["~/.gemini/tmp".into()],
    }
}

fn roots_path() -> PathBuf {
    home().join(".digimon/usage-roots.json")
}

fn root_path(tool: Tool, path: &str) -> RootPath {
    let path = normalized(&expand_path(path));
    let mut files = Vec::new();
    walk_logs(tool, &path, &mut files);
    RootPath { path: path.display().to_string(), exists: path.exists(), files: files.len() as u64 }
}

fn expand_path(path: &str) -> PathBuf {
    if path == "~" { return home(); }
    if let Some(rest) = path.strip_prefix("~/") { return home().join(rest); }
    let path = PathBuf::from(path);
    if path.is_absolute() { path } else { std::env::current_dir().unwrap_or_default().join(path) }
}

fn normalized(path: &Path) -> PathBuf {
    if let Ok(path) = path.canonicalize() { return path; }
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {},
            Component::ParentDir => { result.pop(); },
            component => result.push(component.as_os_str()),
        }
    }
    result
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}
