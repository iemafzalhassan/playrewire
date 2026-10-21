//! Anthropic Messages API (`POST /v1/messages`, SSE streaming).

use crate::{prompt::HintRequest, prompt::SYSTEM_PROMPT, Decoded, ProviderConfig};
use serde_json::{json, Value};

pub const VERSION: &str = "2023-06-01";
pub const MAX_TOKENS: u32 = 300;

pub fn url(cfg: &ProviderConfig) -> String {
    let base = cfg.base_url.trim_end_matches('/');
    if base.ends_with("/v1") { format!("{base}/messages") } else { format!("{base}/v1/messages") }
}

pub fn body(cfg: &ProviderConfig, req: &HintRequest) -> Value {
    // Thinking is off: a 300-token reply budget must not be spent on reasoning.
    json!({
        "model": cfg.model,
        "max_tokens": MAX_TOKENS,
        "stream": true,
        "thinking": {"type": "disabled"},
        "system": SYSTEM_PROMPT,
        "messages": [{"role": "user", "content": req.user_content()}],
    })
}

pub fn decode(data: &str) -> Result<Decoded, String> {
    let v: Value = serde_json::from_str(data).map_err(|e| format!("bad stream event: {e}"))?;
    match v.get("type").and_then(Value::as_str) {
        Some("content_block_delta") => {
            let d = v.get("delta");
            let is_text = d.and_then(|d| d.get("type")).and_then(Value::as_str) == Some("text_delta");
            match d.and_then(|d| d.get("text")).and_then(Value::as_str) {
                Some(t) if is_text && !t.is_empty() => Ok(Decoded::Delta(t.to_string())),
                _ => Ok(Decoded::Skip),
            }
        }
        Some("message_stop") => Ok(Decoded::Done),
        Some("error") => {
            let kind = v.pointer("/error/type").and_then(Value::as_str).unwrap_or("unknown");
            Err(format!("provider error event: {kind}"))
        }
        _ => Ok(Decoded::Skip), // message_start, ping, content_block_start/stop, message_delta
    }
}
