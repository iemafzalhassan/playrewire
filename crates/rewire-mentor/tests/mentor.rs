use httpmock::prelude::*;
use rewire_mentor::*;
use std::time::{Duration, Instant};

fn req() -> HintRequest {
    HintRequest {
        concept: "loops".into(),
        exercise_prompt: "Print 1 to 3".into(),
        code: "for i := 1; i < 3; i++ {}".into(),
        last_error: Some("nothing printed".into()),
        attempt: 2,
    }
}

fn cfg(preset: &str, base: String) -> ProviderConfig {
    ProviderConfig { preset_id: preset.into(), base_url: base, model: "m".into() }
}

async fn collect(
    cfg: &ProviderConfig,
    key: Option<&str>,
) -> (Result<(), MentorError>, Vec<AgUiEvent>) {
    let mut evs = Vec::new();
    let r = hint(cfg, key.map(String::from), &req(), &mut |e| evs.push(e)).await;
    (r, evs)
}

const OPENAI_SSE: &str = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\r\n\r\n\
: keep-alive\r\n\r\n\
data: {\"choices\":[{\"delta\":{\"content\":\"What does\"}}]}\r\n\r\n\
data: {\"choices\":[{\"delta\":{\"content\":\"\"}}]}\r\n\r\n\
data: {\"choices\":[{\"delta\":{\"content\":\" line 1 do?\"}}]}\r\n\r\n\
data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\r\n\r\n\
data: [DONE]\r\n\r\n";

fn kinds(evs: &[AgUiEvent]) -> Vec<&'static str> {
    evs.iter()
        .map(|e| match e {
            AgUiEvent::RunStarted { .. } => "RUN_STARTED",
            AgUiEvent::TextMessageStart { .. } => "START",
            AgUiEvent::TextMessageContent { .. } => "CONTENT",
            AgUiEvent::TextMessageEnd { .. } => "END",
            AgUiEvent::RunFinished { .. } => "RUN_FINISHED",
            AgUiEvent::RunError { .. } => "RUN_ERROR",
        })
        .collect()
}

#[tokio::test]
async fn openai_compat_streams_sse_into_agui_events() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions").header("authorization", "Bearer sk-test");
        then.status(200).header("content-type", "text/event-stream").body(OPENAI_SSE);
    });
    let (r, evs) = collect(&cfg("openai", server.url("/v1")), Some("sk-test")).await;
    m.assert();
    r.unwrap();
    assert_eq!(kinds(&evs), ["RUN_STARTED", "START", "CONTENT", "CONTENT", "END", "RUN_FINISHED"]);
    let text: String = evs
        .iter()
        .filter_map(|e| match e {
            AgUiEvent::TextMessageContent { delta, .. } => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "What does line 1 do?");
    assert!(matches!(&evs[1], AgUiEvent::TextMessageStart { role, .. } if role == "assistant"));
}

#[tokio::test]
async fn openai_compat_without_key_sends_no_auth_header() {
    let server = MockServer::start();
    let with_auth = server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions").header_exists("authorization");
        then.status(500);
    });
    let ok = server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(200).body(OPENAI_SSE);
    });
    let (r, _) = collect(&cfg("ollama", server.url("/v1")), None).await;
    r.unwrap();
    assert_eq!(with_auth.calls(), 0);
    ok.assert();
}

#[tokio::test]
async fn anthropic_streams_into_agui_events() {
    let server = MockServer::start();
    let sse = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{}}\n\n\
event: ping\ndata: {\"type\":\"ping\"}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Look at\"}}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\" line 1.\"}}\n\n\
event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/messages")
            .header("x-api-key", "ak")
            .header("anthropic-version", "2023-06-01")
            .header("user-agent", "Rewire");
        then.status(200).header("content-type", "text/event-stream").body(sse);
    });
    let (r, evs) = collect(&cfg("anthropic", server.base_url()), Some("ak")).await;
    m.assert();
    r.unwrap();
    assert_eq!(kinds(&evs), ["RUN_STARTED", "START", "CONTENT", "CONTENT", "END", "RUN_FINISHED"]);
}

#[tokio::test]
async fn request_contains_no_extra_fields() {
    let r = req();
    let expected = serde_json::to_string(&r).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&expected).unwrap();
    let mut keys: Vec<_> = parsed.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["attempt", "code", "concept", "exercise_prompt", "last_error"]);

    // OpenAI-compatible
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions").is_true(move |req| {
            let v: serde_json::Value = serde_json::from_slice(req.body_ref()).unwrap();
            let msgs = v["messages"].as_array().unwrap();
            msgs.len() == 2
                && msgs[0]["role"] == "system"
                && msgs[1]["role"] == "user"
                && msgs[1]["content"] == serde_json::Value::String(
                    serde_json::to_string(&HintRequest {
                        concept: "loops".into(),
                        exercise_prompt: "Print 1 to 3".into(),
                        code: "for i := 1; i < 3; i++ {}".into(),
                        last_error: Some("nothing printed".into()),
                        attempt: 2,
                    })
                    .unwrap(),
                )
                && v["stream"] == true
                && v.as_object().unwrap().len() == 3
        });
        then.status(200).body(OPENAI_SSE);
    });
    collect(&cfg("openai", server.url("/v1")), Some("k")).await.0.unwrap();
    m.assert();

    // Anthropic: same user content, system at top level, no other message.
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/v1/messages").is_true(|req| {
            let v: serde_json::Value = serde_json::from_slice(req.body_ref()).unwrap();
            let msgs = v["messages"].as_array().unwrap();
            msgs.len() == 1
                && msgs[0]["content"]
                    == serde_json::Value::String(
                        serde_json::to_string(&HintRequest {
                            concept: "loops".into(),
                            exercise_prompt: "Print 1 to 3".into(),
                            code: "for i := 1; i < 3; i++ {}".into(),
                            last_error: Some("nothing printed".into()),
                            attempt: 2,
                        })
                        .unwrap(),
                    )
                && v["system"] == SYSTEM_PROMPT
                && v["max_tokens"] == 300
                && v["stream"] == true
        });
        then.status(200)
            .body("data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"x\"}}\n\ndata: {\"type\":\"message_stop\"}\n\n");
    });
    collect(&cfg("anthropic", server.base_url()), Some("k")).await.0.unwrap();
    m.assert();
}

#[test]
fn system_prompt_exact() {
    assert_eq!(
        SYSTEM_PROMPT,
        "You are Rewire's coding mentor for a beginner who may be nervous about code. Never write the full solution. Reply in at most 3 short sentences. Ask one guiding question or point to one specific line. Use plain words and explain any technical term you use."
    );
}

#[test]
fn timeouts_are_5s_connect_20s_total() {
    assert_eq!(CONNECT_TIMEOUT, Duration::from_secs(5));
    assert_eq!(TOTAL_TIMEOUT, Duration::from_secs(20));
}

fn only_error(evs: &[AgUiEvent], msg: &str) {
    let errs: Vec<_> = evs.iter().filter(|e| matches!(e, AgUiEvent::RunError { .. })).collect();
    assert_eq!(errs.len(), 1, "{evs:?}");
    assert!(matches!(errs[0], AgUiEvent::RunError { message } if message == msg));
    assert!(matches!(evs.last(), Some(AgUiEvent::RunError { .. })));
    assert!(!evs.iter().any(|e| matches!(e, AgUiEvent::RunFinished { .. })));
}

#[tokio::test]
async fn unreachable_emits_run_error_within_timeout() {
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let c = cfg("ollama", format!("http://127.0.0.1:{port}/v1"));
    let t = Instant::now();
    let (r, evs) = collect(&c, None).await;
    assert!(matches!(r, Err(MentorError::Connect(_))));
    assert!(t.elapsed() < TOTAL_TIMEOUT);
    only_error(&evs, MSG_FALLBACK);
    assert_eq!(evs.len(), 1);
}

#[tokio::test]
async fn status_401_emits_run_error_with_plain_message() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(401).body("{\"error\":\"bad key sk-secret-123\"}");
    });
    let (r, evs) = collect(&cfg("openai", server.url("/v1")), Some("sk-secret-123")).await;
    assert!(matches!(r, Err(MentorError::Unauthorized(401))));
    only_error(&evs, "The mentor couldn't connect. Check your key in Settings.");
}

#[tokio::test]
async fn error_text_never_contains_the_key() {
    let key = "sk-secret-123";
    for status in [401u16, 403, 500] {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/chat/completions");
            then.status(status).body(format!("invalid key {key} for you"));
        });
        let (r, evs) = collect(&cfg("openai", server.url("/v1")), Some(key)).await;
        let e = r.unwrap_err();
        let all = format!("{e} | {e:?} | {evs:?}");
        assert!(!all.contains(key), "{all}");
    }
    // A stream chunk that echoes the key in malformed JSON.
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(200).body(format!("data: not json {key}\n\n"));
    });
    let (r, evs) = collect(&cfg("openai", server.url("/v1")), Some(key)).await;
    let all = format!("{:?} | {evs:?}", r.unwrap_err());
    assert!(!all.contains(key), "{all}");
}

#[tokio::test]
async fn other_failures_emit_one_run_error() {
    // 500
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(500);
    });
    let (r, evs) = collect(&cfg("openai", server.url("/v1")), None).await;
    assert!(matches!(r, Err(MentorError::Http { status: 500 })));
    only_error(&evs, MSG_FALLBACK);

    // Malformed stream after RUN_STARTED: still exactly one RUN_ERROR, no RUN_FINISHED.
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(200).body("data: {oops\n\n");
    });
    let (r, evs) = collect(&cfg("openai", server.url("/v1")), None).await;
    assert!(matches!(r, Err(MentorError::Stream(_))));
    assert!(matches!(evs[0], AgUiEvent::RunStarted { .. }));
    only_error(&evs, MSG_FALLBACK);

    // Anthropic error event mid-stream.
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v1/messages");
        then.status(200).body("data: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n");
    });
    let (r, evs) = collect(&cfg("anthropic", server.base_url()), Some("k")).await;
    assert!(r.is_err());
    only_error(&evs, MSG_FALLBACK);

    // Anthropic stream that stops without message_stop.
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v1/messages");
        then.status(200).body("data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n\n");
    });
    let (r, evs) = collect(&cfg("anthropic", server.base_url()), Some("k")).await;
    assert!(r.is_err());
    only_error(&evs, MSG_FALLBACK);

    // Unknown preset.
    let (r, evs) = collect(&cfg("nope", "http://x".into()), None).await;
    assert!(matches!(r, Err(MentorError::UnknownPreset(_))));
    only_error(&evs, MSG_FALLBACK);
}

#[tokio::test]
async fn slow_stream_times_out_at_20s() {
    // Per ruling R12: short timeouts against a delayed mock; constants are checked separately.
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(200).delay(Duration::from_secs(3)).body(OPENAI_SSE);
    });
    let mut evs = Vec::new();
    let t = Instant::now();
    let r = hint_with_timeouts(
        &cfg("openai", server.url("/v1")),
        None,
        &req(),
        &mut |e| evs.push(e),
        Duration::from_secs(1),
        Duration::from_millis(300),
    )
    .await;
    assert!(matches!(r, Err(MentorError::Timeout)));
    assert!(t.elapsed() < Duration::from_secs(2));
    only_error(&evs, MSG_FALLBACK);
}

#[test]
fn presets_have_unique_ids_and_all_spec_providers() {
    let ps = presets();
    let mut ids: Vec<_> = ps.iter().map(|p| p.id).collect();
    let n = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), n, "duplicate preset id");
    for want in [
        "ollama", "llamacpp", "lmstudio", "nvidia-nim", "openrouter", "openai", "anthropic",
        "meta-llama", "deepseek", "minimax", "omniroute", "custom",
    ] {
        assert!(ids.contains(&want), "missing {want}");
    }
    assert_eq!(ids.len(), 12);
    for id in ["ollama", "llamacpp", "lmstudio", "omniroute"] {
        let p = ps.iter().find(|p| p.id == id).unwrap();
        assert!(p.local && !p.needs_key, "{id}");
    }
    let custom = ps.iter().find(|p| p.id == "custom").unwrap();
    assert!(!custom.needs_key && !custom.local);
    let a = ps.iter().find(|p| p.id == "anthropic").unwrap();
    assert_eq!(a.api, ApiKind::AnthropicMessages);
    assert_eq!(a.default_model, "claude-haiku-5-5");
    assert_eq!(a.base_url, "https://api.anthropic.com");
    for p in ps.iter().filter(|p| p.needs_key) {
        assert!(p.base_url.starts_with("https://"), "{}", p.id);
    }
}

#[test]
fn agui_serialization_shape() {
    let e = AgUiEvent::TextMessageContent { message_id: "m1".into(), delta: "hi".into() };
    assert_eq!(
        serde_json::to_string(&e).unwrap(),
        r#"{"type":"TEXT_MESSAGE_CONTENT","messageId":"m1","delta":"hi"}"#
    );
    let e = AgUiEvent::RunStarted { thread_id: "t".into(), run_id: "r".into() };
    assert_eq!(
        serde_json::to_string(&e).unwrap(),
        r#"{"type":"RUN_STARTED","threadId":"t","runId":"r"}"#
    );
    let e = AgUiEvent::RunError { message: "x".into() };
    assert_eq!(serde_json::to_string(&e).unwrap(), r#"{"type":"RUN_ERROR","message":"x"}"#);
    let back: AgUiEvent = serde_json::from_str(r#"{"type":"TEXT_MESSAGE_END","messageId":"m"}"#).unwrap();
    assert_eq!(back, AgUiEvent::TextMessageEnd { message_id: "m".into() });
}

#[test]
fn memory_key_store_returns_none_for_missing_entry() {
    let s = MemoryStore::default();
    assert_eq!(s.get("openai").unwrap(), None);
    s.set("openai", "k").unwrap();
    assert_eq!(s.get("openai").unwrap().as_deref(), Some("k"));
    assert_eq!(KEYRING_SERVICE, "dev.playrewire.rewire");
}

fn assert_no_key_fragment(text: &str, key: &str) {
    let chars: Vec<char> = key.chars().collect();
    for w in chars.windows(8) {
        let frag: String = w.iter().collect();
        assert!(!text.contains(&frag), "leaked fragment {frag:?} in {text}");
    }
}

#[tokio::test]
async fn openai_error_object_echoing_key_never_surfaces() {
    let key = "sk-\"quoted\\key-0123456789abcdefghijklmnopqrstuvwxyz";
    // Place the key across the old 200-char truncation boundary.
    let msg = format!("{}{}", "x".repeat(170), key);
    let chunk = serde_json::json!({"error": {"message": msg, "type": "invalid_request_error"}});
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(200).body(format!("data: {chunk}\n\n"));
    });
    let (r, evs) = collect(&cfg("openai", server.url("/v1")), Some(key)).await;
    let e = r.unwrap_err();
    let all = format!("{e} | {e:?} | {evs:?}");
    assert_no_key_fragment(&all, key);
    assert!(all.contains("invalid_request_error"), "{all}");

    // Non-2xx body echoing the key across the same boundary.
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(500).body(format!("{}{}", "y".repeat(170), key));
    });
    let (r, evs) = collect(&cfg("openai", server.url("/v1")), Some(key)).await;
    let e = r.unwrap_err();
    assert_no_key_fragment(&format!("{e} | {e:?} | {evs:?}"), key);
}

#[tokio::test]
async fn sse_line_without_newline_is_capped() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(200).body(format!("data: {}", "a".repeat(200 * 1024)));
    });
    let (r, evs) = collect(&cfg("openai", server.url("/v1")), None).await;
    assert!(matches!(r, Err(MentorError::Stream(_))));
    only_error(&evs, MSG_FALLBACK);
}

#[tokio::test]
async fn oversized_reply_is_capped() {
    let one = format!(
        "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{}\"}}}}]}}\n\n",
        "z".repeat(32 * 1024)
    );
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(200).body(one.repeat(12)); // 384 KiB of text > 256 KiB
    });
    let (r, evs) = collect(&cfg("openai", server.url("/v1")), None).await;
    assert!(matches!(r, Err(MentorError::Stream(_))));
    only_error(&evs, MSG_FALLBACK);
}
