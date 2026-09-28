use chrono::{DateTime, Duration, Local, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::{self, OpenOptions}, io::Write, path::PathBuf};

pub mod parsers {
    pub mod claude_code;
    pub mod codex;
    pub mod gemini;
}
pub mod store;
pub mod roots;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tool {
    ClaudeCode,
    Codex,
    GeminiCli,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roots {
    pub custom: BTreeMap<Tool, Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootPath {
    pub path: String,
    pub exists: bool,
    pub files: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRoots {
    pub tool: Tool,
    pub defaults: Vec<RootPath>,
    pub custom: Vec<RootPath>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRecord {
    pub key: String,
    pub occurred_at: DateTime<Utc>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_write_tokens: u64,
    pub cache_read_tokens: u64,
    pub model: String,
    pub source: Tool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counter {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_write_tokens: u64,
    pub cache_read_tokens: u64,
    pub total_tokens: u64,
    pub records: u64,
}

impl Counter {
    pub fn add(&mut self, record: &UsageRecord) {
        self.input_tokens += record.input_tokens;
        self.output_tokens += record.output_tokens;
        self.cache_write_tokens += record.cache_write_tokens;
        self.cache_read_tokens += record.cache_read_tokens;
        self.total_tokens += record.input_tokens
            + record.output_tokens
            + record.cache_write_tokens
            + record.cache_read_tokens;
        self.records += 1;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUsage {
    pub tool: Tool,
    pub counter: Counter,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model: String,
    pub tool: Tool,
    pub counter: Counter,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String,
    pub total_tokens: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalUsage {
    pub range: String,
    pub generated_at: DateTime<Utc>,
    pub last_scan_at: Option<DateTime<Utc>>,
    pub scanning: bool,
    pub total: Counter,
    pub by_tool: Vec<ToolUsage>,
    pub by_model: Vec<ModelUsage>,
    pub daily: Vec<DailyUsage>,
    pub warnings: Vec<String>,
}

pub struct Accumulator {
    records: BTreeMap<String, UsageRecord>,
    path: PathBuf,
}

impl Accumulator {
    pub fn load() -> Self {
        let path = default_data_dir().join("records.jsonl");
        let mut accumulator = Self { records: BTreeMap::new(), path };
        if let Ok(contents) = fs::read_to_string(&accumulator.path) {
            for line in contents.lines() {
                if let Ok(record) = serde_json::from_str(line) { accumulator.merge(record); }
            }
        }
        let cutoff = Utc::now() - Duration::days(365);
        let before = accumulator.records.len();
        accumulator.records.retain(|_, record| record.occurred_at >= cutoff);
        if accumulator.records.len() != before { let _ = accumulator.rewrite(); }
        accumulator
    }

    pub fn merge(&mut self, incoming: UsageRecord) -> bool {
        match self.records.get_mut(&incoming.key) {
            None => { self.records.insert(incoming.key.clone(), incoming); true }
            Some(existing) if existing.source == Tool::ClaudeCode && incoming.source == Tool::ClaudeCode => {
                let old = (existing.input_tokens, existing.output_tokens, existing.cache_write_tokens, existing.cache_read_tokens);
                existing.input_tokens = existing.input_tokens.max(incoming.input_tokens);
                existing.output_tokens = existing.output_tokens.max(incoming.output_tokens);
                existing.cache_write_tokens = existing.cache_write_tokens.max(incoming.cache_write_tokens);
                existing.cache_read_tokens = existing.cache_read_tokens.max(incoming.cache_read_tokens);
                old != (existing.input_tokens, existing.output_tokens, existing.cache_write_tokens, existing.cache_read_tokens)
            }
            Some(_) => false,
        }
    }

    pub fn merge_and_append(&mut self, record: UsageRecord) -> std::io::Result<()> {
        if self.merge(record.clone()) {
            if let Some(parent) = self.path.parent() { fs::create_dir_all(parent)?; }
            let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
            serde_json::to_writer(&mut file, &record)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            file.write_all(b"\n")?;
        }
        Ok(())
    }

    pub fn records(&self) -> &BTreeMap<String, UsageRecord> { &self.records }

    pub fn rewrite(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() { fs::create_dir_all(parent)?; }
        let mut file = fs::File::create(&self.path)?;
        for record in self.records.values() {
            serde_json::to_writer(&mut file, record)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            file.write_all(b"\n")?;
        }
        Ok(())
    }
}

pub fn scan_all(roots: &Roots, offsets: &mut store::OffsetStore, accumulator: &mut Accumulator) -> Vec<String> {
    let mut warnings = Vec::new();
    for tool in [Tool::ClaudeCode, Tool::Codex, Tool::GeminiCli] {
        for root in roots.current(tool) {
            let mut files = Vec::new();
            roots::walk_logs(tool, &root, &mut files);
            for path in files {
                match offsets.read_new_lines(&path) {
                    Ok(Some(batch)) => {
                        let records = match tool {
                            Tool::ClaudeCode => batch.lines.iter().filter_map(|line| parsers::claude_code::parse_line(line)).collect(),
                            Tool::Codex => batch.lines.iter().filter_map(|line| parsers::codex::parse_line(line, &path)).collect(),
                            Tool::GeminiCli => parsers::gemini::parse_file(&batch.lines.join("\n"), &path),
                        };
                        for record in records {
                            if let Err(error) = accumulator.merge_and_append(record) {
                                warnings.push(format!("Could not save usage records: {error}"));
                                break;
                            }
                        }
                    }
                    Ok(None) => {},
                    Err(error) => warnings.push(format!("Could not read {}: {error}", path.display())),
                }
            }
        }
    }
    if let Err(error) = accumulator.rewrite() {
        warnings.push(format!("Could not compact usage records: {error}"));
    }
    warnings
}

pub fn aggregate(accumulator: &Accumulator, range: &str, last_scan_at: Option<DateTime<Utc>>, warnings: Vec<String>) -> LocalUsage {
    let today = Local::now().date_naive();
    let first_day = match range { "today" => today, "week" => today - Duration::days(6), _ => chrono::NaiveDate::MIN };
    let mut total = Counter::default();
    let mut by_tool = [Tool::ClaudeCode, Tool::Codex, Tool::GeminiCli].into_iter()
        .map(|tool| (tool, Counter::default())).collect::<BTreeMap<_, _>>();
    let mut by_model = BTreeMap::<(Tool, String), Counter>::new();
    let mut daily = BTreeMap::new();
    for offset in 0..7 { daily.insert(today - Duration::days(6 - offset), 0u64); }
    for record in accumulator.records().values() {
        let date = record.occurred_at.with_timezone(&Local).date_naive();
        if let Some(value) = daily.get_mut(&date) { *value += record.input_tokens + record.output_tokens + record.cache_write_tokens + record.cache_read_tokens; }
        if date < first_day { continue; }
        total.add(record);
        by_tool.get_mut(&record.source).unwrap().add(record);
        by_model.entry((record.source, record.model.clone())).or_default().add(record);
    }
    let mut by_model = by_model.into_iter().map(|((tool, model), counter)| ModelUsage { model, tool, counter }).collect::<Vec<_>>();
    by_model.sort_by(|left, right| right.counter.total_tokens.cmp(&left.counter.total_tokens));
    LocalUsage {
        range: range.to_owned(), generated_at: Utc::now(), last_scan_at, scanning: false, total,
        by_tool: by_tool.into_iter().map(|(tool, counter)| ToolUsage { tool, counter }).collect(), by_model,
        daily: daily.into_iter().map(|(date, total_tokens)| DailyUsage { date: date.to_string(), total_tokens }).collect(), warnings,
    }
}

fn default_data_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(".digimon")
}

#[cfg(test)]
mod dedupe_tests {
    use super::*;

    fn record(key: &str, source: Tool, input: u64, output: u64, write: u64, read: u64) -> UsageRecord {
        UsageRecord { key: key.into(), occurred_at: Utc::now(), input_tokens: input, output_tokens: output, cache_write_tokens: write, cache_read_tokens: read, model: "first".into(), source }
    }

    #[test]
    fn claude_duplicates_take_each_token_field_maximum() {
        let mut accumulator = Accumulator { records: BTreeMap::new(), path: PathBuf::new() };
        accumulator.merge(record("request", Tool::ClaudeCode, 2, 1, 3, 4));
        accumulator.merge(record("request", Tool::ClaudeCode, 1, 202, 2, 3));
        accumulator.merge(record("request", Tool::ClaudeCode, 5, 1, 6, 7));
        let result = accumulator.records().get("request").unwrap();
        assert_eq!((result.input_tokens, result.output_tokens, result.cache_write_tokens, result.cache_read_tokens), (5, 202, 6, 7));
    }

    #[test]
    fn codex_duplicate_key_keeps_the_first_record() {
        let mut accumulator = Accumulator { records: BTreeMap::new(), path: PathBuf::new() };
        accumulator.merge(record("event", Tool::Codex, 1, 2, 3, 4));
        accumulator.merge(record("event", Tool::Codex, 9, 9, 9, 9));
        let result = accumulator.records().get("event").unwrap();
        assert_eq!((result.input_tokens, result.output_tokens, result.cache_write_tokens, result.cache_read_tokens), (1, 2, 3, 4));
    }
}
