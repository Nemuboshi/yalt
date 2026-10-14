package translation

import (
	"strings"
	"testing"
)

func TestReadStreamAcceptsSSEVariantsAndFiltersThinking(t *testing.T) {
	input := strings.Join([]string{
		": comment",
		"data:{\"choices\":[{\"delta\":{\"content\":\"<thi\"}}]}",
		"",
		"data: {\"choices\":[{\"delta\":{\"content\":\"nk>secret</think>译\"}}]}",
		"",
		"data: {\"choices\":[]}",
		"",
		"data: [DONE]",
		"",
	}, "\r\n")
	var got string
	result, err := ReadStream(strings.NewReader(input), func(delta string) error {
		got += delta
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if got != "译" || result.Text != "译" || !result.Completed {
		t.Fatalf("result = %#v, deltas = %q", result, got)
	}
}

func TestReadStreamRejectsUnexpectedEnd(t *testing.T) {
	_, err := ReadStream(strings.NewReader(`data: {"choices":[{"delta":{"content":"partial"}}]}`+"\n\n"), func(string) error { return nil })
	if err == nil {
		t.Fatal("expected incomplete stream error")
	}
}

func TestReadStreamReportsProviderError(t *testing.T) {
	_, err := ReadStream(strings.NewReader(`data: {"error":{"message":"rate limited"}}`+"\n\n"), func(string) error { return nil })
	if err == nil || !strings.Contains(err.Error(), "rate limited") {
		t.Fatalf("error = %v", err)
	}
}

func TestReadResponsesStreamHandlesThinkingAndCompletion(t *testing.T) {
	input := strings.Join([]string{
		"data: {\"type\":\"response.output_text.delta\",\"delta\":\"<think>hidden</think>Visible\"}",
		"",
		"data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}",
		"",
	}, "\n")
	var deltas strings.Builder
	result, err := ReadResponsesStream(strings.NewReader(input), func(delta string) error {
		deltas.WriteString(delta)
		return nil
	})
	if err != nil {
		t.Fatalf("ReadResponsesStream() error = %v", err)
	}
	if result.Text != "Visible" || deltas.String() != "Visible" || !result.Completed {
		t.Errorf("result = %#v, deltas = %q", result, deltas.String())
	}
}

func TestReadResponsesStreamReportsErrors(t *testing.T) {
	tests := []struct {
		name string
		data string
		want string
	}{
		{
			name: "provider error",
			data: "data: {\"type\":\"error\",\"error\":{\"message\":\"rate limited\"}}\n\n",
			want: "provider error: rate limited",
		},
		{
			name: "incomplete",
			data: "data: {\"type\":\"response.incomplete\",\"response\":{\"incomplete_details\":{\"reason\":\"max_output_tokens\"}}}\n\n",
			want: "provider returned an incomplete response: max_output_tokens",
		},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			_, err := ReadResponsesStream(strings.NewReader(test.data), nil)
			if err == nil || err.Error() != test.want {
				t.Fatalf("ReadResponsesStream() error = %v, want %q", err, test.want)
			}
		})
	}
}
