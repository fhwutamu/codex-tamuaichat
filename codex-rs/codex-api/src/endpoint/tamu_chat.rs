use crate::auth::SharedAuthProvider;
use crate::common::ResponseEvent;
use crate::common::ResponseStream;
use crate::endpoint::session::EndpointSession;
use crate::error::ApiError;
use crate::provider::Provider;
use crate::requests::TamuChatRequest;
use codex_client::HttpTransport;
use codex_client::RequestTelemetry;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use http::Method;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::instrument;

pub struct TamuChatClient<T: HttpTransport> {
    session: EndpointSession<T>,
}

impl<T: HttpTransport> TamuChatClient<T> {
    pub fn new(transport: T, provider: Provider, auth: SharedAuthProvider) -> Self {
        Self {
            session: EndpointSession::new(transport, provider, auth),
        }
    }

    pub fn with_telemetry(self, request: Option<Arc<dyn RequestTelemetry>>) -> Self {
        Self {
            session: self.session.with_request_telemetry(request),
        }
    }

    #[instrument(
        name = "tamu_chat.request",
        level = "info",
        skip_all,
        fields(
            transport = "tamu_chat_http",
            http.method = "POST",
            api.path = "chat/completions"
        )
    )]
    pub async fn request(&self, request: TamuChatRequest) -> Result<ResponseStream, ApiError> {
        let response = self
            .session
            .execute(
                Method::POST,
                "chat/completions",
                request.headers,
                Some(request.body),
            )
            .await?;
        let upstream_request_id = response
            .headers
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let response: ChatResponse = serde_json::from_slice(&response.body)
            .map_err(|error| ApiError::Stream(format!("invalid TAMU chat response: {error}")))?;
        let choice = response.choices.into_iter().next().ok_or_else(|| {
            ApiError::Stream("TAMU chat response contained no choices".to_string())
        })?;
        if choice.finish_reason.as_deref() == Some("length") {
            return Err(ApiError::ContextWindowExceeded);
        }

        let end_turn = match choice.finish_reason.as_deref() {
            Some("stop") => Some(true),
            Some("tool_calls") => Some(false),
            Some(_) | None => None,
        };
        let mut events = Vec::new();
        if let Some(content) = choice.message.content.filter(|content| !content.is_empty()) {
            events.extend([
                ResponseEvent::OutputItemAdded(assistant_message("")),
                ResponseEvent::OutputTextDelta(content.clone()),
                ResponseEvent::OutputItemDone(assistant_message(&content)),
            ]);
        }
        for (index, tool_call) in choice.message.tool_calls.into_iter().enumerate() {
            events.push(ResponseEvent::OutputItemDone(ResponseItem::FunctionCall {
                id: None,
                name: tool_call.function.name,
                namespace: None,
                arguments: tool_call.function.arguments,
                call_id: tool_call.id.unwrap_or_else(|| format!("tamu-call-{index}")),
                internal_chat_message_metadata_passthrough: None,
            }));
        }
        events.push(ResponseEvent::Completed {
            response_id: response.id,
            token_usage: None,
            end_turn,
        });

        let (tx_event, rx_event) = mpsc::channel(events.len());
        for event in events {
            let _ = tx_event.try_send(Ok(event));
        }
        drop(tx_event);
        Ok(ResponseStream {
            rx_event,
            upstream_request_id,
        })
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    id: String,
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    finish_reason: Option<String>,
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ChatToolCall>,
}

#[derive(Deserialize)]
struct ChatToolCall {
    id: Option<String>,
    function: ChatFunctionCall,
}

#[derive(Deserialize)]
struct ChatFunctionCall {
    name: String,
    #[serde(default)]
    arguments: String,
}

fn assistant_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}
