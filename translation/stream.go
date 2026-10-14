package translation

import (
	"bufio"
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"strings"
)

const (
	maxSSEEvent   = 1 << 20
	maxOutputSize = 4 << 20
)

type StreamResult struct {
	Text         string
	FinishReason string
	Completed    bool
}

func ReadStream(r io.Reader, onDelta func(string) error) (StreamResult, error) {
	reader := bufio.NewReaderSize(r, 32*1024)
	var event strings.Builder
	var output strings.Builder
	filter := new(ThinkFilter)
	result := StreamResult{}
	for {
		line, err := reader.ReadString('\n')
		if len(line) > 0 {
			line = strings.TrimSuffix(strings.TrimSuffix(line, "\n"), "\r")
			if line == "" {
				if err := handleSSEEvent(event.String(), &result, &output, filter, onDelta); err != nil {
					return result, err
				}
				event.Reset()
			} else if !strings.HasPrefix(line, ":") {
				event.WriteString(line)
				event.WriteByte('\n')
				if event.Len() > maxSSEEvent {
					return result, errors.New("stream event is too large")
				}
			}
		}
		if err != nil {
			if errors.Is(err, io.EOF) {
				if event.Len() > 0 {
					if err := handleSSEEvent(event.String(), &result, &output, filter, onDelta); err != nil {
						return result, err
					}
				}
				if !result.Completed {
					return result, errors.New("stream ended before completion")
				}
				return result, nil
			}
			return result, fmt.Errorf("read stream: %w", err)
		}
	}
}

func ReadResponsesStream(r io.Reader, onDelta func(string) error) (StreamResult, error) {
	reader := bufio.NewReaderSize(r, 32*1024)
	var event strings.Builder
	var output strings.Builder
	filter := new(ThinkFilter)
	result := StreamResult{}
	for {
		line, err := reader.ReadString('\n')
		if len(line) > 0 {
			line = strings.TrimSuffix(strings.TrimSuffix(line, "\n"), "\r")
			if line == "" {
				if err := handleResponsesEvent(event.String(), &result, &output, filter, onDelta); err != nil {
					result.Text = output.String()
					return result, err
				}
				event.Reset()
			} else if !strings.HasPrefix(line, ":") {
				event.WriteString(line)
				event.WriteByte('\n')
				if event.Len() > maxSSEEvent {
					return result, errors.New("stream event is too large")
				}
			}
		}
		if err != nil {
			if errors.Is(err, io.EOF) {
				if event.Len() > 0 {
					if err := handleResponsesEvent(event.String(), &result, &output, filter, onDelta); err != nil {
						result.Text = output.String()
						return result, err
					}
				}
				if !result.Completed {
					result.Text = output.String()
					return result, errors.New("stream ended before completion")
				}
				result.Text = output.String()
				if strings.TrimSpace(result.Text) == "" {
					return result, errors.New("provider returned no output text")
				}
				return result, nil
			}
			result.Text = output.String()
			return result, fmt.Errorf("read stream: %w", err)
		}
	}
}

func handleResponsesEvent(raw string, result *StreamResult, output *strings.Builder, filter *ThinkFilter, onDelta func(string) error) error {
	var data strings.Builder
	for _, line := range strings.Split(raw, "\n") {
		if strings.HasPrefix(line, "data:") {
			value := strings.TrimPrefix(line, "data:")
			data.WriteString(strings.TrimPrefix(value, " "))
			data.WriteByte('\n')
		}
	}
	if data.Len() == 0 {
		return nil
	}
	body := strings.TrimSuffix(data.String(), "\n")
	if body == "[DONE]" {
		return errors.New("Responses stream ended without a completion event")
	}
	var event struct {
		Type     string `json:"type"`
		Delta    string `json:"delta"`
		Response *struct {
			Status string `json:"status"`
			Error  *struct {
				Message string `json:"message"`
			} `json:"error"`
			IncompleteDetails *struct {
				Reason string `json:"reason"`
			} `json:"incomplete_details"`
		} `json:"response"`
		Error *struct {
			Message string `json:"message"`
		} `json:"error"`
	}
	if err := json.Unmarshal([]byte(body), &event); err != nil {
		return fmt.Errorf("invalid Responses stream event: %w", err)
	}
	switch event.Type {
	case "response.output_text.delta":
		text, err := filter.Feed(event.Delta)
		if err != nil {
			return err
		}
		output.WriteString(text)
		if output.Len() > maxOutputSize {
			return errors.New("translation output is too large")
		}
		if text != "" && onDelta != nil {
			return onDelta(text)
		}
	case "response.completed":
		flush, err := filter.Flush()
		if err != nil {
			return err
		}
		if err := appendResponsesText(output, flush, onDelta); err != nil {
			return err
		}
		result.FinishReason = "completed"
		result.Completed = true
	case "response.incomplete":
		result.FinishReason = "incomplete"
		if event.Response != nil && event.Response.IncompleteDetails != nil && event.Response.IncompleteDetails.Reason != "" {
			return fmt.Errorf("provider returned an incomplete response: %s", event.Response.IncompleteDetails.Reason)
		}
		return errors.New("provider returned an incomplete response")
	case "response.failed":
		if event.Response != nil && event.Response.Error != nil {
			return fmt.Errorf("provider error: %s", event.Response.Error.Message)
		}
		return errors.New("provider failed to create a response")
	case "error":
		if event.Error != nil {
			return fmt.Errorf("provider error: %s", event.Error.Message)
		}
		return errors.New("provider returned an error")
	}
	return nil
}

func appendResponsesText(output *strings.Builder, text string, onDelta func(string) error) error {
	output.WriteString(text)
	if output.Len() > maxOutputSize {
		return errors.New("translation output is too large")
	}
	if text != "" && onDelta != nil {
		return onDelta(text)
	}
	return nil
}

func handleSSEEvent(raw string, result *StreamResult, output *strings.Builder, filter *ThinkFilter, onDelta func(string) error) error {
	var data bytes.Buffer
	for _, line := range strings.Split(raw, "\n") {
		if strings.HasPrefix(line, "data:") {
			value := strings.TrimPrefix(line, "data:")
			data.WriteString(strings.TrimPrefix(value, " "))
			data.WriteByte('\n')
		}
	}
	if data.Len() == 0 {
		return nil
	}
	body := strings.TrimSuffix(data.String(), "\n")
	if body == "[DONE]" {
		flush, err := filter.Flush()
		if err != nil {
			return err
		}
		if flush != "" {
			output.WriteString(flush)
			if output.Len() > maxOutputSize {
				return errors.New("translation output is too large")
			}
			if err := onDelta(flush); err != nil {
				return err
			}
		}
		if result.FinishReason == "length" {
			result.Text = output.String()
			return errors.New("provider stopped at the output token limit")
		}
		result.Completed = true
		result.Text = output.String()
		return nil
	}
	var chunk struct {
		Error *struct {
			Message string `json:"message"`
		} `json:"error"`
		Choices []struct {
			Delta struct {
				Content string `json:"content"`
			} `json:"delta"`
			FinishReason *string `json:"finish_reason"`
		} `json:"choices"`
	}
	if err := json.Unmarshal([]byte(body), &chunk); err != nil {
		return fmt.Errorf("invalid stream event: %w", err)
	}
	if chunk.Error != nil {
		return fmt.Errorf("provider error: %s", chunk.Error.Message)
	}
	if len(chunk.Choices) == 0 {
		return nil
	}
	choice := chunk.Choices[0]
	if choice.FinishReason != nil {
		result.FinishReason = *choice.FinishReason
	}
	if choice.Delta.Content == "" {
		return nil
	}
	text, err := filter.Feed(choice.Delta.Content)
	if err != nil {
		return err
	}
	output.WriteString(text)
	if output.Len() > maxOutputSize {
		return errors.New("translation output is too large")
	}
	if text != "" {
		if err := onDelta(text); err != nil {
			return err
		}
	}
	return nil
}
