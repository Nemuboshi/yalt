package translation

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestTranslateAuthorizationHeader(t *testing.T) {
	tests := []struct {
		name   string
		apiKey string
		want   string
	}{
		{name: "key provided", apiKey: "secret-token", want: "Bearer secret-token"},
		{name: "key is trimmed", apiKey: "  secret-token  ", want: "Bearer secret-token"},
		{name: "key omitted", apiKey: "  "},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			var authorization string
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				authorization = r.Header.Get("Authorization")
				w.Header().Set("Content-Type", "application/json")
				fmt.Fprint(w, `{"choices":[{"message":{"content":"translated"},"finish_reason":"stop"}]}`)
			}))
			defer server.Close()

			client := NewClient(server.Client())
			_, err := client.Translate(context.Background(), Request{
				APIBase:   server.URL,
				APIKey:    test.apiKey,
				Model:     "test-model",
				Text:      "hello",
				Target:    "zh",
				MaxTokens: 16,
			}, nil)
			if err != nil {
				t.Fatalf("Translate() error = %v", err)
			}
			if authorization != test.want {
				t.Errorf("Authorization header = %q, want %q", authorization, test.want)
			}
		})
	}
}

func TestTranslateResponsesRequestAndResult(t *testing.T) {
	var gotPath string
	var gotPayload map[string]any
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.Path
		if err := json.NewDecoder(r.Body).Decode(&gotPayload); err != nil {
			t.Fatalf("decode request: %v", err)
		}
		w.Header().Set("Content-Type", "application/json")
		fmt.Fprint(w, `{"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"translated"}]}]}`)
	}))
	defer server.Close()

	client := NewClient(server.Client())
	result, err := client.Translate(context.Background(), Request{
		APIBase:   server.URL,
		APIMethod: APIMethodResponses,
		Model:     "test-model",
		Text:      "hello",
		Target:    "zh",
		MaxTokens: 16,
	}, nil)
	if err != nil {
		t.Fatalf("Translate() error = %v", err)
	}
	if gotPath != "/responses" {
		t.Errorf("request path = %q, want %q", gotPath, "/responses")
	}
	if gotPayload["model"] != "test-model" || gotPayload["max_output_tokens"] != float64(16) || gotPayload["stream"] != false {
		t.Errorf("request payload = %#v", gotPayload)
	}
	if _, ok := gotPayload["input"].(string); !ok {
		t.Errorf("input has type %T, want string", gotPayload["input"])
	}
	if _, ok := gotPayload["messages"]; ok {
		t.Errorf("Responses request must not contain messages")
	}
	if result.Text != "translated" || result.FinishReason != "completed" {
		t.Errorf("result = %#v", result)
	}
}

func TestTranslateResponsesReportsIncompleteResult(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprint(w, `{"status":"incomplete","incomplete_details":{"reason":"max_output_tokens"}}`)
	}))
	defer server.Close()

	_, err := NewClient(server.Client()).Translate(context.Background(), Request{
		APIBase: server.URL, APIMethod: APIMethodResponses, Model: "test-model", Text: "hello", MaxTokens: 16,
	}, nil)
	if err == nil || err.Error() != "response was incomplete: max_output_tokens" {
		t.Fatalf("Translate() error = %v", err)
	}
}

func TestTranslateResponsesStream(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/event-stream")
		fmt.Fprint(w, "data: {\"type\":\"response.output_text.delta\",\"delta\":\"trans\"}\n\n")
		fmt.Fprint(w, "data: {\"type\":\"response.output_text.delta\",\"delta\":\"lated\"}\n\n")
		fmt.Fprint(w, "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n\n")
	}))
	defer server.Close()

	var deltas string
	result, err := NewClient(server.Client()).Translate(context.Background(), Request{
		APIBase: server.URL, APIMethod: APIMethodResponses, Model: "test-model", Text: "hello", MaxTokens: 16, Stream: true,
	}, func(delta string) error {
		deltas += delta
		return nil
	})
	if err != nil {
		t.Fatalf("Translate() error = %v", err)
	}
	if result.Text != "translated" || deltas != result.Text {
		t.Errorf("result = %#v, deltas = %q", result, deltas)
	}
}
