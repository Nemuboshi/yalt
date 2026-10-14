package translation

import (
	"strings"
	"testing"
)

func TestBuildPrompt(t *testing.T) {
	prompt := BuildPrompt("Hello", "zh", "en", "Keep JSON", []Glossary{{Source: "API", Target: "接口"}})
	for _, want := range []string{"英语文本", "中文", "API→接口", "【注意】Keep JSON", "Hello"} {
		if !strings.Contains(prompt, want) {
			t.Fatalf("prompt does not contain %q: %s", want, prompt)
		}
	}
}

func TestParseGlossary(t *testing.T) {
	got, err := ParseGlossary(`alpha: 阿尔法, beta->贝塔`)
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 2 || got[0].Source != "alpha" || got[1].Target != "贝塔" {
		t.Fatalf("unexpected glossary: %#v", got)
	}
}

func TestValidateGlossaryRejectsDuplicateSources(t *testing.T) {
	err := ValidateGlossary([]Glossary{{Source: "term", Target: "one"}, {Source: "TERM", Target: "two"}})
	if err == nil {
		t.Fatal("expected duplicate glossary error")
	}
}
