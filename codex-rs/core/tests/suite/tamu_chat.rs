use anyhow::Result;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use pretty_assertions::assert_eq;
use serde_json::Value;
use std::time::Duration;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::body_string_contains;
use wiremock::matchers::method;
use wiremock::matchers::path;

fn tamu_test_provider(server: &MockServer) -> ModelProviderInfo {
    ModelProviderInfo {
        name: "TAMU AI Chat test".to_string(),
        base_url: Some(format!("{}/openai", server.uri())),
        env_key: None,
        env_key_instructions: None,
        experimental_bearer_token: Some("test-api-key".to_string()),
        auth: None,
        aws: None,
        wire_api: WireApi::TamuChat,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: Some(0),
        stream_max_retries: Some(0),
        stream_idle_timeout_ms: Some(2_000),
        websocket_connect_timeout_ms: None,
        requires_openai_auth: false,
        supports_websockets: false,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tamu_chat_drives_a_shell_tool_round_trip() -> Result<()> {
    skip_if_no_network!(Ok(()));
    let server = MockServer::start().await;

    let tool_response = ResponseTemplate::new(200)
        .insert_header("content-type", "application/json")
        .set_body_json(serde_json::json!({
            "id": "chatcmpl-tool",
            "choices": [{
                "finish_reason": "tool_calls",
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                                "index": 0,
                                "id": "call-tamu-shell",
                                "type": "function",
                                "function": {
                                    "name": "exec_command",
                                    "arguments": "{\"cmd\":\"printf TAMU_TOOL_OK\"}",
                                },
                    }],
                },
            }],
        }));
    Mock::given(method("POST"))
        .and(path("/openai/chat/completions"))
        .and(body_string_contains("Run the TAMU agent test"))
        .respond_with(tool_response)
        .expect(1)
        .mount(&server)
        .await;

    let final_response = ResponseTemplate::new(200)
        .insert_header("content-type", "application/json")
        .set_body_json(serde_json::json!({
            "id": "chatcmpl-final",
            "choices": [{
                "finish_reason": "stop",
                "message": {"role": "assistant", "content": "agent complete"},
            }],
        }));
    Mock::given(method("POST"))
        .and(path("/openai/chat/completions"))
        .and(body_string_contains("TAMU_TOOL_OK"))
        .respond_with(final_response)
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;

    let provider = tamu_test_provider(&server);
    let test = test_codex()
        .with_model("gpt-5.4")
        .with_config(move |config| config.model_provider = provider)
        .build(&server)
        .await?;

    let submit_result = tokio::time::timeout(
        Duration::from_secs(10),
        test.submit_turn("Run the TAMU agent test"),
    )
    .await;
    match submit_result {
        Ok(result) => result?,
        Err(_) => {
            let requests = server.received_requests().await.unwrap_or_default();
            anyhow::bail!(
                "TAMU agent turn timed out after {} mock request(s): {:?}",
                requests.len(),
                requests
                    .iter()
                    .map(|request| request.url.path().to_string())
                    .collect::<Vec<_>>()
            );
        }
    }

    let requests = server.received_requests().await.unwrap_or_default();
    assert_eq!(requests.len(), 2);
    let first: Value = requests[0].body_json()?;
    let second: Value = requests[1].body_json()?;
    assert_eq!(first["model"], "protected.gpt-5.4");
    assert_eq!(first["stream"], "false");
    let tool_names = first["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|tool| tool["function"]["name"].as_str())
        .collect::<Vec<_>>();
    assert!(
        tool_names.contains(&"exec_command"),
        "TAMU request tools: {tool_names:?}"
    );
    assert!(second["messages"].as_array().is_some_and(|messages| {
        messages.iter().any(|message| {
            message["role"] == "tool"
                && message["tool_call_id"] == "call-tamu-shell"
                && message["content"]
                    .as_str()
                    .is_some_and(|content| content.contains("TAMU_TOOL_OK"))
        })
    }));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tamu_chat_drives_two_sequential_tool_rounds() -> Result<()> {
    skip_if_no_network!(Ok(()));
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/openai/chat/completions"))
        .and(body_string_contains("Run two TAMU tool rounds"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(serde_json::json!({
                    "id": "chatcmpl-round-one",
                    "choices": [{
                        "finish_reason": "tool_calls",
                        "message": {"tool_calls": [{
                            "id": "call-tamu-round-one",
                            "type": "function",
                            "function": {
                                "name": "exec_command",
                                "arguments": "{\"cmd\":\"printf TAMU_MULTI_ROUND_ONE\"}",
                            },
                        }]},
                    }],
                })),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/openai/chat/completions"))
        .and(body_string_contains("TAMU_MULTI_ROUND_ONE"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(serde_json::json!({
                    "id": "chatcmpl-round-two",
                    "choices": [{
                        "finish_reason": "tool_calls",
                        "message": {"tool_calls": [{
                            "id": "call-tamu-round-two",
                            "type": "function",
                            "function": {
                                "name": "exec_command",
                                "arguments": "{\"cmd\":\"printf TAMU_MULTI_ROUND_TWO\"}",
                            },
                        }]},
                    }],
                })),
        )
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/openai/chat/completions"))
        .and(body_string_contains("TAMU_MULTI_ROUND_TWO"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(serde_json::json!({
                    "id": "chatcmpl-multi-round-final",
                    "choices": [{
                        "finish_reason": "stop",
                        "message": {"content": "both tool rounds complete"},
                    }],
                })),
        )
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;

    let provider = tamu_test_provider(&server);
    let test = test_codex()
        .with_model("gpt-5.4")
        .with_config(move |config| config.model_provider = provider)
        .build(&server)
        .await?;
    tokio::time::timeout(
        Duration::from_secs(10),
        test.submit_turn("Run two TAMU tool rounds"),
    )
    .await??;

    let requests = server.received_requests().await.unwrap_or_default();
    assert_eq!(requests.len(), 3);
    let second: Value = requests[1].body_json()?;
    let third: Value = requests[2].body_json()?;
    assert!(second["messages"].as_array().is_some_and(|messages| {
        messages.iter().any(|message| {
            message["role"] == "assistant"
                && message["content"] == ""
                && message["tool_calls"][0]["id"] == "call-tamu-round-one"
        })
    }));
    assert!(second["messages"].as_array().is_some_and(|messages| {
        messages.iter().any(|message| {
            message["role"] == "tool"
                && message["tool_call_id"] == "call-tamu-round-one"
                && message["content"]
                    .as_str()
                    .is_some_and(|content| content.contains("TAMU_MULTI_ROUND_ONE"))
        })
    }));
    assert!(third["messages"].as_array().is_some_and(|messages| {
        messages.iter().any(|message| {
            message["role"] == "tool"
                && message["tool_call_id"] == "call-tamu-round-two"
                && message["content"]
                    .as_str()
                    .is_some_and(|content| content.contains("TAMU_MULTI_ROUND_TWO"))
        })
    }));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tamu_chat_preserves_an_existing_protected_model_prefix() -> Result<()> {
    skip_if_no_network!(Ok(()));
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/chat/completions"))
        .and(body_string_contains("Test the protected model prefix"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(serde_json::json!({
                    "id": "chatcmpl-prefixed-model",
                    "choices": [{
                        "finish_reason": "stop",
                        "message": {"role": "assistant", "content": "prefix accepted"},
                    }],
                })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = tamu_test_provider(&server);
    let test = test_codex()
        .with_model("protected.gpt-5.4")
        .with_config(move |config| config.model_provider = provider)
        .build(&server)
        .await?;
    test.submit_turn("Test the protected model prefix").await?;

    let requests = server.received_requests().await.unwrap_or_default();
    assert_eq!(requests.len(), 1);
    let body: Value = requests[0].body_json()?;
    assert_eq!(body["model"], "protected.gpt-5.4");
    assert_eq!(body["stream"], "false");

    Ok(())
}
