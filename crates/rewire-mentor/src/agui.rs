//! AG-UI protocol events the UI consumes. Wire shape: `{"type":"RUN_STARTED","threadId":..}`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum AgUiEvent {
    #[serde(rename = "RUN_STARTED")]
    RunStarted { thread_id: String, run_id: String },
    #[serde(rename = "TEXT_MESSAGE_START")]
    TextMessageStart { message_id: String, role: String },
    #[serde(rename = "TEXT_MESSAGE_CONTENT")]
    TextMessageContent { message_id: String, delta: String },
    #[serde(rename = "TEXT_MESSAGE_END")]
    TextMessageEnd { message_id: String },
    #[serde(rename = "RUN_FINISHED")]
    RunFinished { thread_id: String, run_id: String },
    #[serde(rename = "RUN_ERROR")]
    RunError { message: String },
}

/// Shown when the key was rejected (HTTP 401/403).
pub const MSG_BAD_KEY: &str = "The mentor couldn't connect. Check your key in Settings.";
/// Shown for every other failure. The UI falls back to built-in hints.
pub const MSG_FALLBACK: &str = "The mentor couldn't connect, so here's a built-in hint.";
