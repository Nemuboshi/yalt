//! Server-sent-event framing and provider event decoding.
//!
//! Framing is independent of network read boundaries: the decoder is fed raw
//! byte chunks in any split, including splits inside a UTF-8 sequence, a JSON
//! value, or a reasoning tag. Both API methods are decoded here with their own
//! completion and failure semantics.

use serde::Deserialize;

use super::think::ThinkFilter;

/// Largest single SSE event accepted, in bytes.
const MAX_SSE_EVENT: usize = 1 << 20;
/// Largest accumulated translation output accepted, in bytes.
const MAX_OUTPUT_SIZE: usize = 4 << 20;

/// Outcome of a streamed request.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StreamResult {
    /// Visible text accumulated so far.
    pub text: String,
    /// Provider finish reason, when reported.
    pub finish_reason: String,
    /// Whether the provider signalled completion.
    pub completed: bool,
}

/// Reason a stream ended without a usable result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    /// An event exceeded the framing limit.
    EventTooLarge,
    /// Accumulated output exceeded the output limit.
    OutputTooLarge,
    /// A data payload was not valid JSON for the API in use.
    Malformed(String),
    /// The provider reported an error.
    Provider(String),
    /// The connection ended before the completion marker.
    PrematureEnd,
    /// The provider stopped at the output token limit.
    TokenLimit,
    /// The response ended inside an unfinished reasoning block.
    UnfinishedReasoning,
    /// The read failed.
    Io(String),
    /// The Responses stream ended with `[DONE]` instead of a completion event.
    MissingCompletionEvent,
    /// A completed Responses stream carried no visible text.
    EmptyOutput,
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventTooLarge => f.write_str("stream event is too large"),
            Self::OutputTooLarge => f.write_str("translation output is too large"),
            Self::Malformed(message) => write!(f, "invalid stream event: {message}"),
            Self::Provider(message) => write!(f, "provider error: {message}"),
            Self::PrematureEnd => f.write_str("stream ended before completion"),
            Self::TokenLimit => f.write_str("provider stopped at the output token limit"),
            Self::UnfinishedReasoning => f.write_str("response ended inside a reasoning block"),
            Self::Io(message) => write!(f, "read stream: {message}"),
            Self::MissingCompletionEvent => {
                f.write_str("Responses stream ended without a completion event")
            }
            Self::EmptyOutput => f.write_str("provider returned no output text"),
        }
    }
}

impl std::error::Error for StreamError {}

impl From<super::think::UnfinishedReasoning> for StreamError {
    fn from(_: super::think::UnfinishedReasoning) -> Self {
        Self::UnfinishedReasoning
    }
}

/// Which API contract a stream belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamFormat {
    /// OpenAI-compatible Chat Completions, terminated by `data: [DONE]`.
    ChatCompletions,
    /// OpenAI Responses, terminated by a `response.completed` event.
    Responses,
}

/// Incremental decoder for one streamed response.
///
/// Call [`SseDecoder::push`] with each raw chunk and [`SseDecoder::finish`] once
/// the body ends. Visible text is appended to [`SseDecoder::output`].
pub struct SseDecoder {
    format: StreamFormat,
    /// Bytes received but not yet split into complete lines.
    pending: Vec<u8>,
    /// Lines of the event currently being assembled.
    event: String,
    /// Visible output accumulated across the stream.
    output: String,
    /// Incremental reasoning filter.
    filter: ThinkFilter,
    /// Completion state and finish reason.
    result: StreamResult,
}

impl SseDecoder {
    /// Create a decoder for the given API contract.
    pub fn new(format: StreamFormat) -> Self {
        Self {
            format,
            pending: Vec::new(),
            event: String::new(),
            output: String::new(),
            filter: ThinkFilter::new(),
            result: StreamResult::default(),
        }
    }

    /// Visible text accumulated so far.
    pub fn output(&self) -> &str {
        &self.output
    }

    /// Finish reason and completion state observed so far.
    pub fn result(&self) -> &StreamResult {
        &self.result
    }

    /// Feed a raw chunk, returning the visible text it completes.
    ///
    /// On error, [`SseDecoder::output`] still holds the visible text produced
    /// before the failure so partial results can be preserved.
    pub fn push(&mut self, chunk: &[u8]) -> Result<String, StreamError> {
        self.pending.extend_from_slice(chunk);
        let mut produced = String::new();
        while let Some(newline) = self.pending.iter().position(|byte| *byte == b'\n') {
            let mut line: Vec<u8> = self.pending.drain(..=newline).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            let line = String::from_utf8_lossy(&line).into_owned();
            produced.push_str(&self.push_line(&line)?);
        }
        Ok(produced)
    }

    /// Finish the stream after the body ends.
    ///
    /// Returns the visible text produced by the final partial event. Fails when
    /// the provider never signalled completion.
    pub fn finish(&mut self) -> Result<String, StreamError> {
        let mut produced = String::new();
        if !self.pending.is_empty() {
            let line = String::from_utf8_lossy(&self.pending).into_owned();
            self.pending.clear();
            produced.push_str(&self.push_line(&line)?);
        }
        if !self.event.is_empty() {
            let event = std::mem::take(&mut self.event);
            produced.push_str(&self.handle_event(&event)?);
        }
        if !self.result.completed {
            return Err(StreamError::PrematureEnd);
        }
        if self.format == StreamFormat::Responses && self.output.trim().is_empty() {
            return Err(StreamError::EmptyOutput);
        }
        Ok(produced)
    }

    /// Process one complete line.
    fn push_line(&mut self, line: &str) -> Result<String, StreamError> {
        if line.is_empty() {
            let event = std::mem::take(&mut self.event);
            return self.handle_event(&event);
        }
        if line.starts_with(':') {
            // Comment or keep-alive.
            return Ok(String::new());
        }
        self.event.push_str(line);
        self.event.push('\n');
        if self.event.len() > MAX_SSE_EVENT {
            return Err(StreamError::EventTooLarge);
        }
        Ok(String::new())
    }

    /// Decode one assembled SSE event.
    fn handle_event(&mut self, raw: &str) -> Result<String, StreamError> {
        let Some(body) = data_payload(raw) else {
            return Ok(String::new());
        };
        match self.format {
            StreamFormat::ChatCompletions => self.handle_chat_event(&body),
            StreamFormat::Responses => self.handle_responses_event(&body),
        }
    }

    /// Decode a Chat Completions event.
    fn handle_chat_event(&mut self, body: &str) -> Result<String, StreamError> {
        if body == "[DONE]" {
            let tail = self.filter.flush()?;
            let produced = self.append(&tail)?;
            if self.result.finish_reason == "length" {
                return Err(StreamError::TokenLimit);
            }
            self.result.completed = true;
            self.result.text = self.output.clone();
            return Ok(produced);
        }
        let chunk: ChatChunk = serde_json::from_str(body)
            .map_err(|error| StreamError::Malformed(error.to_string()))?;
        if let Some(error) = chunk.error {
            return Err(StreamError::Provider(error.message));
        }
        let Some(choice) = chunk.choices.into_iter().next() else {
            // Empty choices are metadata, not text.
            return Ok(String::new());
        };
        if let Some(reason) = choice.finish_reason {
            self.result.finish_reason = reason;
        }
        let text = self.filter.feed(&choice.delta.content);
        self.append(&text)
    }

    /// Decode a Responses event.
    fn handle_responses_event(&mut self, body: &str) -> Result<String, StreamError> {
        if body == "[DONE]" {
            return Err(StreamError::MissingCompletionEvent);
        }
        let event: ResponsesEvent = serde_json::from_str(body)
            .map_err(|error| StreamError::Malformed(error.to_string()))?;
        match event.kind.as_str() {
            "response.output_text.delta" => {
                let text = self.filter.feed(&event.delta);
                self.append(&text)
            }
            "response.completed" => {
                let tail = self.filter.flush()?;
                let produced = self.append(&tail)?;
                self.result.finish_reason = "completed".to_owned();
                self.result.completed = true;
                self.result.text = self.output.clone();
                Ok(produced)
            }
            "response.incomplete" => {
                self.result.finish_reason = "incomplete".to_owned();
                match event
                    .response
                    .and_then(|response| response.incomplete_details)
                {
                    Some(details) if !details.reason.is_empty() => {
                        Err(StreamError::Provider(format!(
                            "provider returned an incomplete response: {}",
                            details.reason
                        )))
                    }
                    _ => Err(StreamError::Provider(
                        "provider returned an incomplete response".to_owned(),
                    )),
                }
            }
            "response.failed" => {
                let message = event
                    .response
                    .and_then(|response| response.error)
                    .map(|error| error.message);
                Err(StreamError::Provider(message.unwrap_or_else(|| {
                    "provider failed to create a response".to_owned()
                })))
            }
            "error" => {
                let message = event.error.map(|error| error.message);
                Err(StreamError::Provider(
                    message.unwrap_or_else(|| "provider returned an error".to_owned()),
                ))
            }
            // Metadata and lifecycle events carry no visible text.
            _ => Ok(String::new()),
        }
    }

    /// Append visible text to the output, enforcing the output limit.
    fn append(&mut self, text: &str) -> Result<String, StreamError> {
        if text.is_empty() {
            return Ok(String::new());
        }
        self.output.push_str(text);
        if self.output.len() > MAX_OUTPUT_SIZE {
            return Err(StreamError::OutputTooLarge);
        }
        self.result.text = self.output.clone();
        Ok(text.to_owned())
    }
}

/// Extract the `data:` payload from an assembled event.
///
/// Accepts `data:` with or without a following space and joins multiline data
/// fields with newlines. Returns `None` when the event carries no data.
fn data_payload(raw: &str) -> Option<String> {
    let mut data: Vec<&str> = Vec::new();
    for line in raw.split('\n') {
        if let Some(value) = line.strip_prefix("data:") {
            data.push(value.strip_prefix(' ').unwrap_or(value));
        }
    }
    if data.is_empty() {
        None
    } else {
        Some(data.join("\n"))
    }
}

#[derive(Deserialize)]
struct ProviderError {
    message: String,
}

#[derive(Deserialize)]
struct ChatChunk {
    #[serde(default)]
    error: Option<ProviderError>,
    #[serde(default)]
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    #[serde(default)]
    delta: ChatDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize, Default)]
struct ChatDelta {
    #[serde(default)]
    content: String,
}

#[derive(Deserialize)]
struct ResponsesEvent {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    delta: String,
    #[serde(default)]
    response: Option<ResponsesBody>,
    #[serde(default)]
    error: Option<ProviderError>,
}

#[derive(Deserialize)]
struct ResponsesBody {
    #[serde(default)]
    error: Option<ProviderError>,
    #[serde(default)]
    incomplete_details: Option<IncompleteDetails>,
}

#[derive(Deserialize)]
struct IncompleteDetails {
    #[serde(default)]
    reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decode a whole body fed one byte at a time, to exercise every split.
    fn decode_bytewise(format: StreamFormat, body: &str) -> Result<SseDecoder, StreamError> {
        let mut decoder = SseDecoder::new(format);
        for byte in body.as_bytes() {
            decoder.push(&[*byte])?;
        }
        decoder.finish()?;
        Ok(decoder)
    }

    #[test]
    fn chat_stream_accepts_sse_variants_and_filters_reasoning() {
        let body = [
            ": comment",
            "data:{\"choices\":[{\"delta\":{\"content\":\"<thi\"}}]}",
            "",
            "data: {\"choices\":[{\"delta\":{\"content\":\"nk>secret</think>译\"}}]}",
            "",
            "data: {\"choices\":[]}",
            "",
            "data: [DONE]",
            "",
        ]
        .join("\r\n");
        let decoder = decode_bytewise(StreamFormat::ChatCompletions, &body).unwrap();
        assert_eq!(decoder.output(), "译");
        assert!(decoder.result().completed);
    }

    #[test]
    fn chat_stream_rejects_unexpected_end() {
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n";
        let mut decoder = SseDecoder::new(StreamFormat::ChatCompletions);
        decoder.push(body.as_bytes()).unwrap();
        assert_eq!(decoder.finish(), Err(StreamError::PrematureEnd));
        // Partial output is preserved for the caller.
        assert_eq!(decoder.output(), "partial");
    }

    #[test]
    fn chat_stream_reports_provider_error() {
        let body = "data: {\"error\":{\"message\":\"rate limited\"}}\n\n";
        let mut decoder = SseDecoder::new(StreamFormat::ChatCompletions);
        let error = decoder.push(body.as_bytes()).unwrap_err();
        assert_eq!(error, StreamError::Provider("rate limited".into()));
        assert_eq!(error.to_string(), "provider error: rate limited");
    }

    #[test]
    fn chat_stream_reports_token_limit() {
        // The body ends immediately after the final data line, with no trailing
        // blank line, so the last event is dispatched at end of stream.
        let body = [
            "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"},\"finish_reason\":\"length\"}]}",
            "",
            "data: [DONE]",
            "",
        ]
        .join("\n");
        let mut decoder = SseDecoder::new(StreamFormat::ChatCompletions);
        // The partial text arrives before the limit is reported.
        assert_eq!(decoder.push(body.as_bytes()).unwrap(), "partial");
        assert_eq!(decoder.finish(), Err(StreamError::TokenLimit));
        assert_eq!(decoder.output(), "partial");
    }

    #[test]
    fn chat_stream_rejects_malformed_data() {
        let body = "data: {not json}\n\n";
        let mut decoder = SseDecoder::new(StreamFormat::ChatCompletions);
        assert!(matches!(
            decoder.push(body.as_bytes()),
            Err(StreamError::Malformed(_))
        ));
    }

    #[test]
    fn responses_stream_handles_reasoning_and_completion() {
        let body = [
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"<think>hidden</think>Visible\"}",
            "",
            "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}",
            "",
        ]
        .join("\n");
        let decoder = decode_bytewise(StreamFormat::Responses, &body).unwrap();
        assert_eq!(decoder.output(), "Visible");
        assert!(decoder.result().completed);
    }

    #[test]
    fn responses_stream_reports_errors() {
        let cases = [
            (
                "data: {\"type\":\"error\",\"error\":{\"message\":\"rate limited\"}}\n\n",
                "provider error: rate limited",
            ),
            (
                "data: {\"type\":\"response.incomplete\",\"response\":{\"incomplete_details\":{\"reason\":\"max_output_tokens\"}}}\n\n",
                "provider error: provider returned an incomplete response: max_output_tokens",
            ),
            (
                "data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"message\":\"boom\"}}}\n\n",
                "provider error: boom",
            ),
        ];
        for (body, want) in cases {
            let mut decoder = SseDecoder::new(StreamFormat::Responses);
            let error = decoder.push(body.as_bytes()).unwrap_err();
            assert_eq!(error.to_string(), want);
        }
    }

    #[test]
    fn responses_stream_rejects_done_without_completion() {
        let mut decoder = SseDecoder::new(StreamFormat::Responses);
        let error = decoder.push(b"data: [DONE]\n\n").unwrap_err();
        assert_eq!(error, StreamError::MissingCompletionEvent);
    }

    #[test]
    fn responses_stream_requires_output() {
        let body =
            "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n\n";
        let mut decoder = SseDecoder::new(StreamFormat::Responses);
        decoder.push(body.as_bytes()).unwrap();
        assert_eq!(decoder.finish(), Err(StreamError::EmptyOutput));
    }

    #[test]
    fn responses_stream_ignores_metadata_events() {
        let body = [
            "data: {\"type\":\"response.created\",\"response\":{\"status\":\"in_progress\"}}",
            "",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hi\"}",
            "",
            "data: {\"type\":\"response.completed\"}",
            "",
        ]
        .join("\n");
        let decoder = decode_bytewise(StreamFormat::Responses, &body).unwrap();
        assert_eq!(decoder.output(), "hi");
    }

    #[test]
    fn framing_is_independent_of_chunk_boundaries() {
        let body =
            "data: {\"choices\":[{\"delta\":{\"content\":\"héllo 世界\"}}]}\n\ndata: [DONE]\n\n";
        // Feed in awkward slices that split UTF-8 and JSON.
        let bytes = body.as_bytes();
        let mut decoder = SseDecoder::new(StreamFormat::ChatCompletions);
        for chunk in bytes.chunks(3) {
            decoder.push(chunk).unwrap();
        }
        decoder.finish().unwrap();
        assert_eq!(decoder.output(), "héllo 世界");
    }

    #[test]
    fn accepts_data_field_without_a_space() {
        let body = "data:{\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\n\ndata:[DONE]\n\n";
        let decoder = decode_bytewise(StreamFormat::ChatCompletions, body).unwrap();
        assert_eq!(decoder.output(), "x");
    }

    #[test]
    fn rejects_an_oversized_event() {
        let mut decoder = SseDecoder::new(StreamFormat::ChatCompletions);
        let oversized = format!("data: {}\n", "x".repeat(MAX_SSE_EVENT + 1));
        assert_eq!(
            decoder.push(oversized.as_bytes()),
            Err(StreamError::EventTooLarge)
        );
    }

    #[test]
    fn rejects_oversized_output() {
        let mut decoder = SseDecoder::new(StreamFormat::ChatCompletions);
        let payload = "y".repeat(64 * 1024);
        let mut error = None;
        for _ in 0..(MAX_OUTPUT_SIZE / payload.len() + 2) {
            let body =
                format!("data: {{\"choices\":[{{\"delta\":{{\"content\":\"{payload}\"}}}}]}}\n\n");
            if let Err(failure) = decoder.push(body.as_bytes()) {
                error = Some(failure);
                break;
            }
        }
        assert_eq!(error, Some(StreamError::OutputTooLarge));
    }

    #[test]
    fn keeps_visible_text_when_reasoning_is_unfinished() {
        let body = [
            "data: {\"choices\":[{\"delta\":{\"content\":\"visible<think>hidden\"}}]}",
            "",
            "data: [DONE]",
            "",
        ]
        .join("\n");
        let mut decoder = SseDecoder::new(StreamFormat::ChatCompletions);
        decoder.push(body.as_bytes()).unwrap();
        assert_eq!(decoder.finish(), Err(StreamError::UnfinishedReasoning));
        // The visible text seen before the unfinished block is retained.
        assert_eq!(decoder.output(), "visible");
    }

    #[test]
    fn crlf_and_lf_bodies_decode_identically() {
        let lines = [
            "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}",
            "",
            "data: {\"choices\":[{\"delta\":{\"content\":\"b\"}}]}",
            "",
            "data: [DONE]",
            "",
        ];
        let lf = decode_bytewise(StreamFormat::ChatCompletions, &lines.join("\n")).unwrap();
        let crlf = decode_bytewise(StreamFormat::ChatCompletions, &lines.join("\r\n")).unwrap();
        assert_eq!(lf.output(), "ab");
        assert_eq!(crlf.output(), "ab");
    }
}
