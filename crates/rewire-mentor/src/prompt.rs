//! The mentor's system prompt and the one user message it ever sends.

use serde::{Deserialize, Serialize};

/// Exact system prompt. Do not edit without updating the spec.
pub const SYSTEM_PROMPT: &str = "You are Rewire's coding mentor for a beginner who may be nervous about code. Never write the full solution. Reply in at most 3 short sentences. Ask one guiding question or point to one specific line. Use plain words and explain any technical term you use.";

/// Everything the mentor is allowed to send. No personal data, ever:
/// only these five fields go into the user message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HintRequest {
    pub concept: String,
    pub exercise_prompt: String,
    pub code: String,
    pub last_error: Option<String>,
    pub attempt: u32,
}

impl HintRequest {
    /// The user message content: the JSON serialization of this struct, nothing else.
    pub fn user_content(&self) -> String {
        serde_json::to_string(self).expect("HintRequest always serializes")
    }
}
