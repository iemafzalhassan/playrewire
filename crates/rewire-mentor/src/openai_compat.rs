//! OpenAI-compatible `/chat/completions` request and chunk decoding.

use crate::{prompt::SYSTEM_PROMPT, prompt::HintRequest, Decoded, ProviderConfig};
use serde_json::{json, Value};

pub fn url(cfg: &ProviderConfig) -> String {
    format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'))
}

pub fn body(cfg: &ProviderConfig, req: &HintRequest) -> Value {
    json!({
        "model": cfg.model,
        "stream": true,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": req.user_content()},
        ],
    })
}

pub fn decode(data: &str) -> Result<Decoded, String> {
    if data.trim() == "[DONE]" {
        return Ok(Decoded::Done);
    }
    let v: Value = serde_json::from_str(data).map_err(|e| format!("bad stream chunk: {e}"))?;
    if let Some(err) = v.get("error") {
        return Err(format!("provider error: {}", crate::safe_code(err.get("type").or_else(|| err.get("code")))));
    }
    let choice = v.pointer("/choices/0");
    if let Some(text) = choice.and_then(|c| c.pointer("/delta/content")).and_then(Value::as_str)
        && !text.is_empty()
    {
        return Ok(Decoded::Delta(text.to_string()));
    }
    let finished = choice.and_then(|c| c.get("finish_reason")).is_some_and(|f| !f.is_null());
    Ok(if finished { Decoded::Finished } else { Decoded::Skip })
}
