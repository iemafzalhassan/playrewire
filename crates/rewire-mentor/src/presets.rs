//! Provider presets. Base URLs and default models were checked against each
//! provider's docs on 2026-10-21; see `default_model` notes in the task report.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKind {
    OpenAiCompatible,
    AnthropicMessages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub base_url: &'static str,
    pub api: ApiKind,
    pub needs_key: bool,
    pub local: bool,
    /// A sensible starting model; the player can change it.
    pub default_model: &'static str,
}

const fn p(
    id: &'static str,
    label: &'static str,
    base_url: &'static str,
    api: ApiKind,
    needs_key: bool,
    local: bool,
    default_model: &'static str,
) -> Preset {
    Preset { id, label, base_url, api, needs_key, local, default_model }
}

use ApiKind::{AnthropicMessages as Anth, OpenAiCompatible as Oai};

static PRESETS: [Preset; 12] = [
    p("ollama", "Ollama (on this computer)", "http://localhost:11434/v1", Oai, false, true, "llama3.2:3b"),
    p("llamacpp", "llama.cpp (on this computer)", "http://localhost:9931/v1", Oai, false, true, "local"),
    p("lmstudio", "LM Studio (on this computer)", "http://localhost:1234/v1", Oai, false, true, "local-model"),
    p("nvidia-nim", "NVIDIA NIM", "https://integrate.api.nvidia.com/v1", Oai, true, false, "meta/llama-3.2-3b-instruct"),
    p("openrouter", "OpenRouter", "https://openrouter.ai/api/v1", Oai, true, false, "openai/gpt-5-mini"),
    p("openai", "OpenAI", "https://api.openai.com/v1", Oai, true, false, "gpt-5-mini"),
    p("anthropic", "Anthropic", "https://api.anthropic.com", Anth, true, false, "claude-haiku-5-5"),
    p("meta-llama", "Meta Llama API", "https://api.llama.com/compat/v1", Oai, true, false, "Llama-4-Maverick-17B-128E-Instruct-FP8"),
    p("deepseek", "DeepSeek", "https://api.deepseek.com", Oai, true, false, "deepseek-flash"),
    p("minimax", "MiniMax", "https://api.minimax.io/v1", Oai, true, false, "MiniMax-M3.1-Flash"),
    p("omniroute", "OmniRoute (on this computer)", "http://localhost:20128/v1", Oai, false, true, "auto"),
    p("custom", "Custom (any OpenAI-compatible server)", "", Oai, false, false, ""),
];

pub fn presets() -> &'static [Preset] {
    &PRESETS
}
