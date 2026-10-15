//! HTTP transport and API-specific payloads and results.
//!
//! Payload fields, endpoint suffixes, authorization, and buffered result
//! handling match the previous implementation. Automatic redirects are
//! disabled so requests and credentials are never silently forwarded.

use std::fmt;

use serde::Deserialize;
use serde_json::json;

use super::Cancel;
use super::prompt::{self, Glossary};
use super::stream::{SseDecoder, StreamError, StreamFormat};
use super::think::ThinkFilter;

/// Largest buffered response body accepted, in bytes.
const MAX_BUFFERED_BODY: usize = 4 << 20;

/// Largest error body read for a diagnostic message, in bytes.
const MAX_ERROR_BODY: usize = 64 * 1024;

/// Provider API contract used for a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ApiMethod {
    /// OpenAI-compatible Chat Completions.
    #[default]
    #[serde(rename = "chat/completions")]
    ChatCompletions,
    /// OpenAI Responses.
    #[serde(rename = "responses")]
    Responses,
}

impl ApiMethod {
    /// Endpoint suffix and serialized settings value for this method.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChatCompletions => "chat/completions",
            Self::Responses => "responses",
        }
    }

    /// Streaming contract for this method.
    fn stream_format(self) -> StreamFormat {
        match self {
            Self::ChatCompletions => StreamFormat::ChatCompletions,
            Self::Responses => StreamFormat::Responses,
        }
    }
}

impl fmt::Display for ApiMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A fully resolved translation request.
///
/// `source` and `target` are resolved codes, never `auto`.
#[derive(Debug, Clone)]
pub struct Request {
    /// Provider base URL, without the endpoint suffix.
    pub api_base: String,
    /// API contract to use.
    pub api_method: ApiMethod,
    /// Optional bearer token.
    pub api_key: String,
    /// Provider model identifier.
    pub model: String,
    /// Text to translate.
    pub text: String,
    /// Resolved source language code, or `auto`.
    pub source: String,
    /// Resolved target language code.
    pub target: String,
    /// Extra instructions appended to the prompt.
    pub instruction: String,
    /// Ordered glossary terms.
    pub glossary: Vec<Glossary>,
    /// Maximum output tokens.
    pub max_tokens: u32,
    /// Sampling temperature.
    pub temperature: f64,
    /// Whether to request a streamed response.
    pub stream: bool,
    /// Whether to ask the provider to suppress reasoning output.
    pub disable_think: bool,
    /// User-Agent header value.
    pub user_agent: String,
}

/// A completed translation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TranslationResult {
    /// Visible translated text.
    pub text: String,
    /// Provider finish reason, when reported.
    pub finish_reason: String,
}

/// Reason a request failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// The input was empty.
    EmptyInput,
    /// The request was cancelled.
    Cancelled,
    /// The request exceeded its deadline.
    Deadline,
    /// The provider returned a non-success status.
    Status { status: u16, body: String },
    /// The connection or body read failed.
    Transport(String),
    /// The response body was not valid for the API in use.
    InvalidResponse(String),
    /// The provider reported an error, or the result was unusable.
    Provider(String),
    /// A streaming protocol failure.
    Stream(StreamError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => f.write_str("input text is empty"),
            Self::Cancelled => f.write_str("translation stopped"),
            Self::Deadline => f.write_str("request deadline exceeded"),
            Self::Status { status, body } if body.is_empty() => {
                write!(f, "provider returned HTTP {status}")
            }
            Self::Status { status, body } => {
                write!(f, "provider returned HTTP {status}: {body}")
            }
            Self::Transport(message) => write!(f, "request failed: {message}"),
            Self::InvalidResponse(message) => write!(f, "invalid provider response: {message}"),
            Self::Provider(message) => write!(f, "provider error: {message}"),
            Self::Stream(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<StreamError> for ClientError {
    fn from(error: StreamError) -> Self {
        Self::Stream(error)
    }
}

impl From<super::think::UnfinishedReasoning> for ClientError {
    fn from(_: super::think::UnfinishedReasoning) -> Self {
        Self::Stream(StreamError::UnfinishedReasoning)
    }
}

/// Shared HTTP client, reused across requests.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
}

impl Client {
    /// Build a client that never follows redirects.
    pub fn new() -> Result<Self, ClientError> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| ClientError::Transport(error.to_string()))?;
        Ok(Self { http })
    }

    /// Wrap an existing HTTP client.
    pub fn with_http(http: reqwest::Client) -> Self {
        Self { http }
    }

    /// Perform a translation, streaming visible text to `on_delta` as it arrives.
    ///
    /// On failure the callback has still received every visible fragment
    /// produced before the failure, so callers can preserve partial output.
    pub async fn translate(
        &self,
        request: &Request,
        cancel: &Cancel,
        mut on_delta: impl FnMut(&str),
    ) -> Result<TranslationResult, ClientError> {
        if request.text.trim().is_empty() {
            return Err(ClientError::EmptyInput);
        }
        let prompt = prompt::build_prompt(
            &request.text,
            &request.target,
            &request.source,
            &request.instruction,
            &request.glossary,
        );
        let payload = build_payload(request, &prompt);
        let url = format!(
            "{}/{}",
            request.api_base.trim_end_matches('/'),
            request.api_method
        );

        let send = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream, application/json")
            .header("User-Agent", &request.user_agent)
            .json(&payload)
            .send();
        let response = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(ClientError::Cancelled),
            response = send => response.map_err(map_reqwest_error)?,
        };

        let status = response.status();
        if !status.is_success() {
            let body = read_limited(response, MAX_ERROR_BODY, cancel).await?;
            return Err(ClientError::Status {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&body).trim().to_owned(),
            });
        }

        if request.stream {
            self.read_stream(request, response, cancel, &mut on_delta)
                .await
        } else {
            self.read_buffered(request, response, cancel, &mut on_delta)
                .await
        }
    }

    /// Read and decode a streamed response.
    async fn read_stream(
        &self,
        request: &Request,
        mut response: reqwest::Response,
        cancel: &Cancel,
        on_delta: &mut impl FnMut(&str),
    ) -> Result<TranslationResult, ClientError> {
        let mut decoder = SseDecoder::new(request.api_method.stream_format());
        let mut failure: Option<ClientError> = None;
        loop {
            let next = tokio::select! {
                biased;
                () = cancel.cancelled() => {
                    failure = Some(ClientError::Cancelled);
                    break;
                }
                next = response.chunk() => next,
            };
            match next {
                Ok(Some(chunk)) => match decoder.push(chunk.as_ref()) {
                    Ok(text) => emit(&text, on_delta),
                    Err(error) => {
                        failure = Some(error.into());
                        break;
                    }
                },
                Ok(None) => break,
                Err(error) => {
                    failure = Some(map_reqwest_error(error));
                    break;
                }
            }
        }
        // Always finish the decoder so buffered visible text is flushed and a
        // truncated final event is not silently dropped.
        let finish = decoder.finish();
        match (failure, finish) {
            (Some(error), _) => Err(error),
            (None, Err(error)) => Err(error.into()),
            (None, Ok(text)) => {
                emit(&text, on_delta);
                Ok(TranslationResult {
                    text: decoder.output().to_owned(),
                    finish_reason: decoder.result().finish_reason.clone(),
                })
            }
        }
    }

    /// Read and decode a buffered response.
    async fn read_buffered(
        &self,
        request: &Request,
        response: reqwest::Response,
        cancel: &Cancel,
        on_delta: &mut impl FnMut(&str),
    ) -> Result<TranslationResult, ClientError> {
        let body = read_limited(response, MAX_BUFFERED_BODY, cancel).await?;
        let body = String::from_utf8_lossy(&body);
        let result = match request.api_method {
            ApiMethod::ChatCompletions => parse_chat_result(&body)?,
            ApiMethod::Responses => parse_responses_result(&body)?,
        };
        emit(&result.text, on_delta);
        Ok(result)
    }
}

/// Emit a fragment to the caller when it is non-empty.
fn emit(text: &str, on_delta: &mut impl FnMut(&str)) {
    if !text.is_empty() {
        on_delta(text);
    }
}

/// Read a response body, bounded and cancellable.
async fn read_limited(
    mut response: reqwest::Response,
    limit: usize,
    cancel: &Cancel,
) -> Result<Vec<u8>, ClientError> {
    let mut body = Vec::new();
    loop {
        let next = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(ClientError::Cancelled),
            next = response.chunk() => next,
        };
        match next {
            Ok(Some(chunk)) => {
                if body.len() + chunk.len() > limit {
                    body.extend_from_slice(&chunk[..limit.saturating_sub(body.len())]);
                    break;
                }
                body.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(error) => return Err(map_reqwest_error(error)),
        }
    }
    Ok(body)
}

/// Map a transport failure onto a client error.
fn map_reqwest_error(error: reqwest::Error) -> ClientError {
    if error.is_timeout() {
        ClientError::Deadline
    } else {
        ClientError::Transport(error.to_string())
    }
}

/// Build the JSON payload for a request.
fn build_payload(request: &Request, prompt: &str) -> serde_json::Value {
    match request.api_method {
        ApiMethod::ChatCompletions => json!({
            "model": request.model,
            "messages": [{ "role": "user", "content": prompt }],
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "stream": request.stream,
            "chat_template_kwargs": { "enable_thinking": !request.disable_think },
        }),
        ApiMethod::Responses => json!({
            "model": request.model,
            "input": prompt,
            "max_output_tokens": request.max_tokens,
            "stream": request.stream,
            "temperature": request.temperature,
        }),
    }
}

/// Parse a buffered Chat Completions response.
fn parse_chat_result(body: &str) -> Result<TranslationResult, ClientError> {
    let response: ChatResponse = serde_json::from_str(body)
        .map_err(|error| ClientError::InvalidResponse(error.to_string()))?;
    if let Some(error) = response.error {
        return Err(ClientError::Provider(error.message));
    }
    let Some(choice) = response.choices.into_iter().next() else {
        return Err(ClientError::Provider(
            "provider response contains no choices".to_owned(),
        ));
    };
    let mut filter = ThinkFilter::new();
    let mut text = filter.feed(&choice.message.content);
    let tail = filter.flush()?;
    text.push_str(&tail);
    Ok(TranslationResult {
        text,
        finish_reason: choice.finish_reason,
    })
}

/// Parse a buffered Responses result.
fn parse_responses_result(body: &str) -> Result<TranslationResult, ClientError> {
    let response: ResponsesResponse = serde_json::from_str(body)
        .map_err(|error| ClientError::InvalidResponse(error.to_string()))?;
    if let Some(error) = response.error {
        return Err(ClientError::Provider(error.message));
    }
    if response.status == "incomplete" {
        let reason = match response.incomplete_details {
            Some(details) if !details.reason.is_empty() => {
                format!(
                    "provider returned an incomplete response: {}",
                    details.reason
                )
            }
            _ => "provider returned an incomplete response".to_owned(),
        };
        return Err(ClientError::Provider(reason));
    }
    let mut text = response.output_text;
    if text.is_empty() {
        for item in response.output {
            if item.kind != "message" {
                continue;
            }
            for content in item.content {
                if content.kind == "output_text" {
                    text.push_str(&content.text);
                }
            }
        }
    }
    if text.trim().is_empty() {
        return Err(ClientError::Provider(
            "provider returned no output text".to_owned(),
        ));
    }
    Ok(TranslationResult {
        text,
        finish_reason: response.status,
    })
}

#[derive(Deserialize)]
struct ProviderError {
    message: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    error: Option<ProviderError>,
    #[serde(default)]
    choices: Vec<ChatResponseChoice>,
}

#[derive(Deserialize)]
struct ChatResponseChoice {
    #[serde(default)]
    message: ChatResponseMessage,
    #[serde(default)]
    finish_reason: String,
}

#[derive(Deserialize, Default)]
struct ChatResponseMessage {
    #[serde(default)]
    content: String,
}

#[derive(Deserialize)]
struct ResponsesResponse {
    #[serde(default)]
    status: String,
    #[serde(default)]
    error: Option<ProviderError>,
    #[serde(default)]
    incomplete_details: Option<IncompleteDetails>,
    #[serde(default)]
    output_text: String,
    #[serde(default)]
    output: Vec<ResponsesItem>,
}

#[derive(Deserialize)]
struct IncompleteDetails {
    #[serde(default)]
    reason: String,
}

#[derive(Deserialize)]
struct ResponsesItem {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    content: Vec<ResponsesContent>,
}

#[derive(Deserialize)]
struct ResponsesContent {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(method: ApiMethod, stream: bool) -> Request {
        Request {
            api_base: "https://example.com/v1".into(),
            api_method: method,
            api_key: String::new(),
            model: "test-model".into(),
            text: "Hello".into(),
            source: "en".into(),
            target: "zh".into(),
            instruction: String::new(),
            glossary: Vec::new(),
            max_tokens: 1024,
            temperature: 0.0,
            stream,
            disable_think: true,
            user_agent: "yalt/test".into(),
        }
    }

    #[test]
    fn chat_payload_uses_messages_and_max_tokens() {
        let payload = build_payload(&request(ApiMethod::ChatCompletions, true), "PROMPT");
        assert_eq!(payload["messages"][0]["role"], "user");
        assert_eq!(payload["messages"][0]["content"], "PROMPT");
        assert_eq!(payload["max_tokens"], 1024);
        assert_eq!(payload["stream"], true);
        assert_eq!(payload["model"], "test-model");
        // Reasoning preference is expressed as a chat template flag.
        assert_eq!(payload["chat_template_kwargs"]["enable_thinking"], false);
        assert!(payload.get("max_output_tokens").is_none());
    }

    #[test]
    fn responses_payload_uses_input_and_max_output_tokens() {
        let payload = build_payload(&request(ApiMethod::Responses, false), "PROMPT");
        assert_eq!(payload["input"], "PROMPT");
        assert_eq!(payload["max_output_tokens"], 1024);
        assert_eq!(payload["stream"], false);
        assert!(payload.get("messages").is_none());
        assert!(payload.get("max_tokens").is_none());
    }

    #[test]
    fn reasoning_preference_flips_with_the_setting() {
        let mut req = request(ApiMethod::ChatCompletions, true);
        req.disable_think = false;
        let payload = build_payload(&req, "P");
        assert_eq!(payload["chat_template_kwargs"]["enable_thinking"], true);
    }

    #[test]
    fn parses_a_buffered_chat_result() {
        let body = r#"{"choices":[{"message":{"content":"你好"},"finish_reason":"stop"}]}"#;
        let result = parse_chat_result(body).unwrap();
        assert_eq!(result.text, "你好");
        assert_eq!(result.finish_reason, "stop");
    }

    #[test]
    fn buffered_chat_filters_reasoning() {
        let body = r#"{"choices":[{"message":{"content":"<think>hidden</think>visible"}}]}"#;
        assert_eq!(parse_chat_result(body).unwrap().text, "visible");
    }

    #[test]
    fn buffered_chat_reports_provider_errors() {
        let error = parse_chat_result(r#"{"error":{"message":"boom"}}"#).unwrap_err();
        assert_eq!(error, ClientError::Provider("boom".into()));
    }

    #[test]
    fn buffered_chat_rejects_empty_choices() {
        let error = parse_chat_result(r#"{"choices":[]}"#).unwrap_err();
        assert!(error.to_string().contains("no choices"), "{error}");
    }

    #[test]
    fn buffered_chat_rejects_malformed_json() {
        assert!(matches!(
            parse_chat_result("not json"),
            Err(ClientError::InvalidResponse(_))
        ));
    }

    #[test]
    fn parses_a_buffered_responses_result() {
        let body = r#"{"status":"completed","output_text":"你好"}"#;
        let result = parse_responses_result(body).unwrap();
        assert_eq!(result.text, "你好");
        assert_eq!(result.finish_reason, "completed");
    }

    #[test]
    fn responses_result_falls_back_to_output_items() {
        let body = r#"{"status":"completed","output":[
            {"type":"reasoning"},
            {"type":"message","content":[
                {"type":"output_text","text":"你好"},
                {"type":"refusal","text":"ignored"}
            ]}
        ]}"#;
        assert_eq!(parse_responses_result(body).unwrap().text, "你好");
    }

    #[test]
    fn responses_result_reports_incomplete() {
        let body = r#"{"status":"incomplete","incomplete_details":{"reason":"max_output_tokens"}}"#;
        let error = parse_responses_result(body).unwrap_err();
        assert!(error.to_string().contains("max_output_tokens"), "{error}");
    }

    #[test]
    fn responses_result_rejects_empty_output() {
        let error = parse_responses_result(r#"{"status":"completed"}"#).unwrap_err();
        assert!(error.to_string().contains("no output text"), "{error}");
    }
}
