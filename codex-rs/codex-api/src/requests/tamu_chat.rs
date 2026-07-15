use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::models::plaintext_agent_message_content;
use http::HeaderMap;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;

const PROTECTED_MODEL_PREFIX: &str = "protected.";

/// Request payload and headers for TAMU AI Chat's Chat Completions endpoint.
#[derive(Clone, Debug)]
pub struct TamuChatRequest {
    pub body: Value,
    pub headers: HeaderMap,
}

pub struct TamuChatRequestBuilder<'a> {
    model: &'a str,
    instructions: &'a str,
    input: &'a [ResponseItem],
    tools: &'a [Value],
    parallel_tool_calls: bool,
}

impl<'a> TamuChatRequestBuilder<'a> {
    pub fn new(
        model: &'a str,
        instructions: &'a str,
        input: &'a [ResponseItem],
        tools: &'a [Value],
    ) -> Self {
        Self {
            model,
            instructions,
            input,
            tools,
            parallel_tool_calls: false,
        }
    }

    pub fn parallel_tool_calls(mut self, parallel_tool_calls: bool) -> Self {
        self.parallel_tool_calls = parallel_tool_calls;
        self
    }

    pub fn build(self) -> TamuChatRequest {
        let mut messages = Vec::new();
        if !self.instructions.is_empty() {
            messages.push(json!({"role": "system", "content": self.instructions}));
        }

        for item in self.input {
            match item {
                ResponseItem::Message { role, content, .. } => {
                    messages.push(chat_message(role, content));
                }
                ResponseItem::AgentMessage {
                    author,
                    recipient,
                    content,
                    ..
                } => {
                    if let Some(content) = plaintext_agent_message_content(content) {
                        messages.push(json!({
                            "role": "user",
                            "content": format!("[{author} -> {recipient}]\n{content}"),
                        }));
                    }
                }
                ResponseItem::FunctionCall {
                    name,
                    arguments,
                    call_id,
                    ..
                } => {
                    let tool_call = json!({
                        "id": call_id,
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": arguments,
                        }
                    });
                    push_tool_call_message(&mut messages, tool_call);
                }
                ResponseItem::FunctionCallOutput {
                    call_id, output, ..
                } => {
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call_id,
                        "content": function_output_content(output),
                    }));
                }
                ResponseItem::AdditionalTools { .. }
                | ResponseItem::Reasoning { .. }
                | ResponseItem::LocalShellCall { .. }
                | ResponseItem::ToolSearchCall { .. }
                | ResponseItem::CustomToolCall { .. }
                | ResponseItem::CustomToolCallOutput { .. }
                | ResponseItem::ToolSearchOutput { .. }
                | ResponseItem::WebSearchCall { .. }
                | ResponseItem::ImageGenerationCall { .. }
                | ResponseItem::Compaction { .. }
                | ResponseItem::CompactionTrigger { .. }
                | ResponseItem::ContextCompaction { .. }
                | ResponseItem::Other => {}
            }
        }

        let model = self
            .model
            .strip_prefix(PROTECTED_MODEL_PREFIX)
            .unwrap_or(self.model);
        let mut body = Map::from_iter([
            (
                "model".to_string(),
                Value::String(format!("{PROTECTED_MODEL_PREFIX}{model}")),
            ),
            ("stream".to_string(), Value::String("false".to_string())),
            ("messages".to_string(), Value::Array(messages)),
        ]);
        if !self.tools.is_empty() {
            body.insert("tools".to_string(), Value::Array(self.tools.to_vec()));
            body.insert("tool_choice".to_string(), Value::String("auto".to_string()));
            body.insert(
                "parallel_tool_calls".to_string(),
                Value::Bool(self.parallel_tool_calls),
            );
        }

        TamuChatRequest {
            body: Value::Object(body),
            headers: HeaderMap::new(),
        }
    }
}

fn chat_message(role: &str, content: &[ContentItem]) -> Value {
    let mut text = String::new();
    let mut parts = Vec::new();
    let mut saw_image = false;

    for item in content {
        match item {
            ContentItem::InputText { text: part } | ContentItem::OutputText { text: part } => {
                text.push_str(part);
                parts.push(json!({"type": "text", "text": part}));
            }
            ContentItem::InputImage { image_url, .. } => {
                saw_image = true;
                parts.push(json!({
                    "type": "image_url",
                    "image_url": {"url": image_url},
                }));
            }
        }
    }

    if role == "assistant" || !saw_image {
        json!({"role": role, "content": text})
    } else {
        json!({"role": role, "content": parts})
    }
}

fn function_output_content(output: &codex_protocol::models::FunctionCallOutputPayload) -> Value {
    match &output.body {
        FunctionCallOutputBody::Text(content) => Value::String(content.clone()),
        FunctionCallOutputBody::ContentItems(items) => Value::Array(
            items
                .iter()
                .filter_map(|item| match item {
                    FunctionCallOutputContentItem::InputText { text } => {
                        Some(json!({"type": "text", "text": text}))
                    }
                    FunctionCallOutputContentItem::InputImage { image_url, .. } => Some(json!({
                        "type": "image_url",
                        "image_url": {"url": image_url},
                    })),
                    FunctionCallOutputContentItem::EncryptedContent { .. } => None,
                })
                .collect(),
        ),
    }
}

fn push_tool_call_message(messages: &mut Vec<Value>, tool_call: Value) {
    if let Some(Value::Object(message)) = messages.last_mut()
        && message.get("role").and_then(Value::as_str) == Some("assistant")
        && message.get("content").and_then(Value::as_str) == Some("")
        && let Some(tool_calls) = message.get_mut("tool_calls").and_then(Value::as_array_mut)
    {
        tool_calls.push(tool_call);
        return;
    }

    messages.push(json!({
        "role": "assistant",
        "content": "",
        "tool_calls": [tool_call],
    }));
}
