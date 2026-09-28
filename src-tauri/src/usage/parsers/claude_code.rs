use crate::usage::{Tool, UsageRecord};
use chrono::{DateTime, Utc};
use serde_json::Value;

pub fn parse_line(line: &str) -> Option<UsageRecord> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("type")?.as_str()? != "assistant" {
        return None;
    }
    let message = value.get("message")?;
    let usage = message.get("usage")?;
    Some(UsageRecord {
        key: value.get("requestId").and_then(Value::as_str)
            .or_else(|| value.get("uuid").and_then(Value::as_str))?.to_owned(),
        occurred_at: value.get("timestamp")?.as_str()?.parse::<DateTime<Utc>>().ok()?,
        input_tokens: number(usage, "input_tokens"),
        output_tokens: number(usage, "output_tokens"),
        cache_write_tokens: number(usage, "cache_creation_input_tokens"),
        cache_read_tokens: number(usage, "cache_read_input_tokens"),
        model: message.get("model").and_then(Value::as_str).unwrap_or_default().to_owned(),
        source: Tool::ClaudeCode,
    })
}

fn number(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::parse_line;

    #[test]
    fn parses_spec_sample() {
        let line = r#"{"type":"assistant","timestamp":"2026-09-22T07:11:40.470Z","requestId":"req_011CfJ4WuTzMKMszZeTAiaMG","uuid":"c79b9e80-94f7-4834-aea6-891d9ba2ae4e","message":{"role":"assistant","model":"claude-sonnet-5","usage":{"input_tokens":2,"output_tokens":44,"cache_creation_input_tokens":53119,"cache_read_input_tokens":0}}}"#;
        let record = parse_line(line).unwrap();
        assert_eq!(record.key, "req_011CfJ4WuTzMKMszZeTAiaMG");
        assert_eq!((record.input_tokens, record.output_tokens, record.cache_write_tokens, record.cache_read_tokens), (2, 44, 53119, 0));
    }

    #[test]
    fn ignores_usage_iterations() {
        let line = r#"{"type":"assistant","timestamp":"2026-09-22T07:11:40.470Z","requestId":"req_iterations","message":{"role":"assistant","model":"claude-sonnet-5","usage":{"input_tokens":2,"output_tokens":44,"cache_creation_input_tokens":7,"cache_read_input_tokens":8,"iterations":[{"input_tokens":2000,"output_tokens":4000,"cache_creation_input_tokens":6000,"cache_read_input_tokens":8000}]}}}"#;
        let record = parse_line(line).unwrap();
        assert_eq!((record.input_tokens, record.output_tokens, record.cache_write_tokens, record.cache_read_tokens), (2, 44, 7, 8));
    }
}
