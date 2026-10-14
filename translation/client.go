package translation

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

type Request struct {
	APIBase      string
	APIMethod    string
	APIKey       string
	Model        string
	Text         string
	Source       string
	Target       string
	Instruction  string
	Glossary     []Glossary
	MaxTokens    int
	Temperature  float64
	Stream       bool
	DisableThink bool
	UserAgent    string
}

const (
	APIMethodChatCompletions = "chat/completions"
	APIMethodResponses       = "responses"
)

type Result struct {
	Text         string
	FinishReason string
}

type Client struct {
	httpClient *http.Client
}

func NewClient(httpClient *http.Client) *Client {
	if httpClient == nil {
		httpClient = &http.Client{}
	}
	return &Client{httpClient: httpClient}
}

func (c *Client) Translate(ctx context.Context, request Request, onDelta func(string) error) (Result, error) {
	if strings.TrimSpace(request.Text) == "" {
		return Result{}, errors.New("input text is empty")
	}
	prompt := BuildPrompt(request.Text, request.Target, request.Source, request.Instruction, request.Glossary)
	method := request.APIMethod
	if method == "" {
		method = APIMethodChatCompletions
	}
	var payload map[string]any
	switch method {
	case APIMethodChatCompletions:
		payload = map[string]any{
			"model":                request.Model,
			"messages":             []map[string]string{{"role": "user", "content": prompt}},
			"temperature":          request.Temperature,
			"max_tokens":           request.MaxTokens,
			"stream":               request.Stream,
			"chat_template_kwargs": map[string]bool{"enable_thinking": !request.DisableThink},
		}
	case APIMethodResponses:
		payload = map[string]any{
			"model":             request.Model,
			"input":             prompt,
			"max_output_tokens": request.MaxTokens,
			"stream":            request.Stream,
			"temperature":       request.Temperature,
		}
	default:
		return Result{}, fmt.Errorf("unsupported API method %q", method)
	}
	body, err := json.Marshal(payload)
	if err != nil {
		return Result{}, fmt.Errorf("encode request: %w", err)
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, strings.TrimRight(request.APIBase, "/")+"/"+method, strings.NewReader(string(body)))
	if err != nil {
		return Result{}, fmt.Errorf("create request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "text/event-stream, application/json")
	req.Header.Set("User-Agent", request.UserAgent)
	if apiKey := strings.TrimSpace(request.APIKey); apiKey != "" {
		req.Header.Set("Authorization", "Bearer "+apiKey)
	}
	resp, err := c.httpClient.Do(req)
	if err != nil {
		return Result{}, fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		data, _ := io.ReadAll(io.LimitReader(resp.Body, 64*1024))
		return Result{}, fmt.Errorf("provider returned HTTP %d: %s", resp.StatusCode, strings.TrimSpace(string(data)))
	}
	if request.Stream {
		var stream StreamResult
		if method == APIMethodResponses {
			stream, err = ReadResponsesStream(resp.Body, onDelta)
		} else {
			stream, err = ReadStream(resp.Body, onDelta)
		}
		if err != nil {
			return Result{Text: stream.Text, FinishReason: stream.FinishReason}, err
		}
		return Result{Text: stream.Text, FinishReason: stream.FinishReason}, nil
	}
	if method == APIMethodResponses {
		return readResponsesResult(resp.Body, onDelta)
	}
	data, err := io.ReadAll(io.LimitReader(resp.Body, 4<<20))
	if err != nil {
		return Result{}, fmt.Errorf("read response: %w", err)
	}
	var response struct {
		Error *struct {
			Message string `json:"message"`
		} `json:"error"`
		Choices []struct {
			Message struct {
				Content string `json:"content"`
			} `json:"message"`
			FinishReason string `json:"finish_reason"`
		} `json:"choices"`
	}
	if err := json.Unmarshal(data, &response); err != nil {
		return Result{}, fmt.Errorf("invalid provider response: %w", err)
	}
	if response.Error != nil {
		return Result{}, fmt.Errorf("provider error: %s", response.Error.Message)
	}
	if len(response.Choices) == 0 {
		return Result{}, errors.New("provider response contains no choices")
	}
	filter := new(ThinkFilter)
	text, err := filter.Feed(response.Choices[0].Message.Content)
	if err != nil {
		return Result{}, err
	}
	tail, err := filter.Flush()
	if err != nil {
		return Result{Text: text, FinishReason: response.Choices[0].FinishReason}, err
	}
	text += tail
	if onDelta != nil && text != "" {
		if err := onDelta(text); err != nil {
			return Result{}, err
		}
	}
	return Result{Text: text, FinishReason: response.Choices[0].FinishReason}, nil
}

func readResponsesResult(body io.Reader, onDelta func(string) error) (Result, error) {
	data, err := io.ReadAll(io.LimitReader(body, 4<<20))
	if err != nil {
		return Result{}, fmt.Errorf("read response: %w", err)
	}
	var response struct {
		Status string `json:"status"`
		Error  *struct {
			Message string `json:"message"`
		} `json:"error"`
		IncompleteDetails *struct {
			Reason string `json:"reason"`
		} `json:"incomplete_details"`
		OutputText string `json:"output_text"`
		Output     []struct {
			Type    string `json:"type"`
			Content []struct {
				Type string `json:"type"`
				Text string `json:"text"`
			} `json:"content"`
		} `json:"output"`
	}
	if err := json.Unmarshal(data, &response); err != nil {
		return Result{}, fmt.Errorf("decode response: %w", err)
	}
	if response.Error != nil {
		return Result{}, fmt.Errorf("provider error: %s", response.Error.Message)
	}
	if response.Status == "incomplete" {
		reason := "response was incomplete"
		if response.IncompleteDetails != nil && response.IncompleteDetails.Reason != "" {
			reason = "response was incomplete: " + response.IncompleteDetails.Reason
		}
		return Result{}, errors.New(reason)
	}
	text := response.OutputText
	if text == "" {
		var output strings.Builder
		for _, item := range response.Output {
			if item.Type != "message" {
				continue
			}
			for _, content := range item.Content {
				if content.Type == "output_text" {
					output.WriteString(content.Text)
				}
			}
		}
		text = output.String()
	}
	if strings.TrimSpace(text) == "" {
		return Result{}, errors.New("provider returned no output text")
	}
	if onDelta != nil {
		if err := onDelta(text); err != nil {
			return Result{}, err
		}
	}
	return Result{Text: text, FinishReason: response.Status}, nil
}

func NewHTTPClient(timeout time.Duration) *http.Client {
	return &http.Client{Timeout: timeout}
}
