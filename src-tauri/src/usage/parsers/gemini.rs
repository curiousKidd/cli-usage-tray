use crate::usage::{Tool, UsageRecord};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::Path;

pub fn parse_file(contents: &str, path: &Path) -> Vec<UsageRecord> {
    if let Ok(value) = serde_json::from_str::<Value>(contents) {
        return values(value)
            .into_iter()
            .enumerate()
            .filter_map(|(index, value)| parse_value(&value, path, index + 1))
            .collect();
    }
    contents
        .lines()
        .enumerate()
        .filter_map(|(index, line)| serde_json::from_str(line).ok()
            .and_then(|value| parse_value(&value, path, index + 1)))
        .collect()
}

fn values(value: Value) -> Vec<Value> {
    match value {
        Value::Array(values) => values,
        value => vec![value],
    }
}

fn parse_value(value: &Value, path: &Path, line: usize) -> Option<UsageRecord> {
    let tokens = value.get("tokens")?;
    if !tokens.is_object() {
        return None;
    }
    Some(UsageRecord {
        key: value
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned).unwrap_or_else(|| format!("{}#{line}", path.display())),
        occurred_at: value.get("timestamp")?.as_str()?.parse::<DateTime<Utc>>().ok()?,
        input_tokens: number(tokens, "input"),
        output_tokens: number(tokens, "output"),
        cache_write_tokens: 0,
        cache_read_tokens: number(tokens, "cached"),
        model: value
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("gemini")
            .to_owned(),
        source: Tool::GeminiCli,
    })
}

fn number(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::parse_file;
    use crate::usage::{roots::walk_logs, Tool};
    use std::{
        fs,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn parses_spec_sample() {
        let line = r#"{ "id": "sample", "timestamp": "2026-09-22T07:11:40.470Z", "model": "gemini-2.5-pro", "tokens": { "input": 1200, "output": 340, "cached": 800 } }"#;
        let records = parse_file(line, Path::new("/tmp/session.json"));
        assert_eq!(records.len(), 1);
        assert_eq!(
            (
                records[0].input_tokens,
                records[0].output_tokens,
                records[0].cache_write_tokens,
                records[0].cache_read_tokens
            ),
            (1200, 340, 0, 800)
        );
    }

    #[test]
    fn scans_json_and_jsonl_recursively() {
        let dir = std::env::temp_dir().join(format!(
            "gemini-parser-{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        fs::create_dir_all(dir.join("nested")).unwrap();
        fs::write(dir.join("session.json"), r#"{"timestamp":"2026-09-22T07:11:40Z","tokens":{"input":1}}"#).unwrap();
        fs::write(
            dir.join("nested/session.jsonl"),
            "{\"timestamp\":\"2026-09-22T07:11:41Z\",\"tokens\":{\"output\":2}}\n{\"timestamp\":\"2026-09-22T07:11:42Z\",\"tokens\":{\"cached\":3}}",
        ).unwrap();
        let mut files = Vec::new();
        walk_logs(Tool::GeminiCli, &dir, &mut files);
        let records: usize = files.iter()
            .filter_map(|path| fs::read_to_string(path).ok().map(|contents| parse_file(&contents, path).len()))
            .sum();
        assert_eq!(records, 3);
        fs::remove_dir_all(dir).unwrap();
    }
}
