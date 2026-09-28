use crate::usage::{Tool, UsageRecord};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

pub fn parse_line(line: &str, path: &Path) -> Option<UsageRecord> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("type")?.as_str()? != "event_msg" || value.pointer("/payload/type")?.as_str()? != "token_count" {
        return None;
    }
    let usage = value.pointer("/payload/info/last_token_usage")?;
    let timestamp = value.get("timestamp")?.as_str()?;
    let mut hasher = DefaultHasher::new();
    line.hash(&mut hasher);
    Some(UsageRecord {
        key: format!("{}#{timestamp}#{:016x}", path.display(), hasher.finish()),
        occurred_at: timestamp.parse::<DateTime<Utc>>().ok()?,
        input_tokens: number(usage, "input_tokens"),
        output_tokens: number(usage, "output_tokens"),
        cache_write_tokens: number(usage, "cache_write_input_tokens"),
        cache_read_tokens: number(usage, "cached_input_tokens"),
        model: "codex".to_owned(),
        source: Tool::Codex,
    })
}

fn number(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::parse_line;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::path::Path;

    #[test]
    fn parses_spec_sample_using_last_usage_only() {
        let line = r#"{"timestamp":"2026-09-03T08:11:14.190Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":21697,"cached_input_tokens":11136,"cache_write_input_tokens":0,"output_tokens":162,"reasoning_output_tokens":0,"total_tokens":21859},"total_token_usage":{"input_tokens":999999,"cached_input_tokens":999999,"cache_write_input_tokens":999999,"output_tokens":999999},"model_context_window":258400}}}"#;
        let path = Path::new("/tmp/rollout.jsonl");
        let record = parse_line(line, path).unwrap();
        let mut hasher = DefaultHasher::new();
        line.hash(&mut hasher);
        assert_eq!(record.key, format!("{}#2026-09-03T08:11:14.190Z#{:016x}", path.display(), hasher.finish()));
        assert_eq!((record.input_tokens, record.output_tokens, record.cache_write_tokens, record.cache_read_tokens), (21697, 162, 0, 11136));
    }
}
