//! rewire-mentor: a hint from the player's own model, streamed as AG-UI events.
//!
//! Privacy: the request carries only `HintRequest` fields plus the key in an
//! auth header. The key is never placed in an error or log string.

mod agui;
mod anthropic;
mod keys;
mod openai_compat;
mod presets;
mod prompt;
mod sse;

pub use agui::{AgUiEvent, MSG_BAD_KEY, MSG_FALLBACK};
pub use keys::{load_key, store_key, KeyStore, KeyringStore, MemoryStore, SERVICE as KEYRING_SERVICE};
pub use presets::{presets, ApiKind, Preset};
pub use prompt::{HintRequest, SYSTEM_PROMPT};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const TOTAL_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub preset_id: String,
    pub base_url: String,
    pub model: String,
}

/// Technical detail for logs and tests. Never contains the API key.
#[derive(Debug, thiserror::Error)]
pub enum MentorError {
    #[error("unknown provider preset: {0}")]
    UnknownPreset(String),
    #[error("could not reach the provider: {0}")]
    Connect(String),
    #[error("the provider took too long to answer")]
    Timeout,
    #[error("the provider rejected the key (HTTP {0})")]
    Unauthorized(u16),
    #[error("the provider answered HTTP {status}")]
    Http { status: u16 },
    #[error("malformed reply stream: {0}")]
    Stream(String),
    #[error("keychain problem: {0}")]
    Keychain(String),
}

pub(crate) enum Decoded {
    Delta(String),
    Finished, // provider said the choice is finished, [DONE] may still follow
    Done,
    Skip,
}

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Hard caps so a hostile or broken server cannot flood memory within the time budget.
pub(crate) const MAX_SSE_BUFFER: usize = 64 * 1024;
pub(crate) const MAX_REPLY_BYTES: usize = 256 * 1024;

/// Keep only a short provider error type/code made of safe characters; never free text.
pub(crate) fn safe_code(v: Option<&serde_json::Value>) -> String {
    match v.and_then(serde_json::Value::as_str) {
        Some(c) if (1..=64).contains(&c.len()) && c.chars().all(|ch| ch.is_ascii_alphanumeric() || "_.-".contains(ch)) => c.to_string(),
        _ => "unknown".to_string(),
    }
}

/// Remove the key from text (full string first, then truncate) before it reaches an error.
fn redact(text: &str, key: Option<&str>) -> String {
    let t = match key.filter(|k| !k.is_empty()) {
        Some(k) => text.replace(k, "[redacted]"),
        None => text.to_string(),
    };
    t.chars().take(200).collect()
}

pub async fn hint(
    cfg: &ProviderConfig,
    api_key: Option<String>,
    req: &HintRequest,
    sink: &mut (dyn FnMut(AgUiEvent) + Send),
) -> Result<(), MentorError> {
    hint_with_timeouts(cfg, api_key, req, sink, CONNECT_TIMEOUT, TOTAL_TIMEOUT).await
}

#[doc(hidden)]
pub async fn hint_with_timeouts(
    cfg: &ProviderConfig,
    api_key: Option<String>,
    req: &HintRequest,
    sink: &mut (dyn FnMut(AgUiEvent) + Send),
    connect: Duration,
    total: Duration,
) -> Result<(), MentorError> {
    let result = match tokio::time::timeout(total, run(cfg, api_key.as_deref(), req, sink, connect)).await {
        Ok(r) => r,
        Err(_) => Err(MentorError::Timeout),
    };
    if let Err(e) = &result {
        let message = match e {
            MentorError::Unauthorized(_) => MSG_BAD_KEY,
            _ => MSG_FALLBACK,
        };
        sink(AgUiEvent::RunError { message: message.to_string() });
    }
    result
}

#[derive(Default)]
struct Progress {
    total: usize,
    got_text: bool,
    finished: bool,
    done: bool,
}

async fn run(
    cfg: &ProviderConfig,
    key: Option<&str>,
    req: &HintRequest,
    sink: &mut (dyn FnMut(AgUiEvent) + Send),
    connect: Duration,
) -> Result<(), MentorError> {
    let preset = presets()
        .iter()
        .find(|p| p.id == cfg.preset_id)
        .ok_or_else(|| MentorError::UnknownPreset(cfg.preset_id.clone()))?;
    let kind = preset.api;
    let key = key.filter(|k| !k.is_empty());

    let client = reqwest::Client::builder()
        .user_agent("Rewire")
        .connect_timeout(connect)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| MentorError::Connect(redact(&e.to_string(), key)))?;

    let (url, body) = match kind {
        ApiKind::OpenAiCompatible => (openai_compat::url(cfg), openai_compat::body(cfg, req)),
        ApiKind::AnthropicMessages => (anthropic::url(cfg), anthropic::body(cfg, req)),
    };
    let mut rb = client.post(url).header("Accept", "text/event-stream")
        .header("Content-Type", "application/json")
        .body(body.to_string());
    match kind {
        ApiKind::OpenAiCompatible => {
            if let Some(k) = key {
                rb = rb.bearer_auth(k);
            }
        }
        ApiKind::AnthropicMessages => {
            rb = rb.header("anthropic-version", anthropic::VERSION);
            if let Some(k) = key {
                rb = rb.header("x-api-key", k);
            }
        }
    }

    let resp = rb.send().await.map_err(|e| MentorError::Connect(redact(&e.without_url().to_string(), key)))?;
    let status = resp.status();
    if matches!(status.as_u16(), 401 | 403) {
        return Err(MentorError::Unauthorized(status.as_u16()));
    }
    if !status.is_success() {
        // The body is never read or surfaced: it is provider text and could echo the key.
        return Err(MentorError::Http { status: status.as_u16() });
    }

    let (thread_id, run_id, message_id) = (uuid(), uuid(), uuid());
    sink(AgUiEvent::RunStarted { thread_id: thread_id.clone(), run_id: run_id.clone() });
    sink(AgUiEvent::TextMessageStart { message_id: message_id.clone(), role: "assistant".into() });

    let mut parser = sse::SseParser::default();
    let mut stream = resp.bytes_stream();
    let mut st = Progress::default();
    let handle = |ev: sse::SseEvent,
                      st: &mut Progress,
                      sink: &mut (dyn FnMut(AgUiEvent) + Send)|
     -> Result<(), MentorError> {
        let decoded = match kind {
            ApiKind::OpenAiCompatible => openai_compat::decode(&ev.data),
            ApiKind::AnthropicMessages => anthropic::decode(&ev.data),
        }
        .map_err(|m| MentorError::Stream(redact(&m, key)))?;
        match decoded {
            Decoded::Delta(delta) => {
                st.total += delta.len();
                if st.total > MAX_REPLY_BYTES {
                    return Err(MentorError::Stream("reply too large".into()));
                }
                st.got_text = true;
                sink(AgUiEvent::TextMessageContent { message_id: message_id.clone(), delta });
            }
            Decoded::Finished => st.finished = true,
            Decoded::Done => st.done = true,
            Decoded::Skip => {}
        }
        Ok(())
    };

    while !st.done {
        let Some(chunk) = stream.next().await else { break };
        let chunk = chunk.map_err(|e| MentorError::Stream(redact(&e.without_url().to_string(), key)))?;
        let events = parser.feed(&chunk).map_err(|_| MentorError::Stream("stream line too large".into()))?;
        for ev in events {
            handle(ev, &mut st, sink)?;
            if st.done {
                break;
            }
        }
    }
    if !st.done
        && let Some(ev) = parser.finish()
    {
        handle(ev, &mut st, sink)?;
    }
    let Progress { got_text, finished, done, .. } = st;

    // Anthropic must say message_stop; OpenAI-style servers may just close after finish_reason.
    let complete = done || (kind == ApiKind::OpenAiCompatible && (finished || got_text));
    if !complete {
        return Err(MentorError::Stream("stream ended early".into()));
    }
    if !got_text {
        return Err(MentorError::Stream("the model sent an empty reply".into()));
    }
    sink(AgUiEvent::TextMessageEnd { message_id });
    sink(AgUiEvent::RunFinished { thread_id, run_id });
    Ok(())
}
