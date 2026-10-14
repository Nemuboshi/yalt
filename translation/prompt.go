package translation

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"strings"
)

type Glossary struct {
	Source string `json:"source"`
	Target string `json:"target"`
}

func ValidateGlossary(glossary []Glossary) error {
	seen := make(map[string]struct{}, len(glossary))
	for _, term := range glossary {
		source, target := strings.TrimSpace(term.Source), strings.TrimSpace(term.Target)
		if source == "" || target == "" {
			return errors.New("glossary terms must have source and target values")
		}
		if _, ok := seen[strings.ToLower(source)]; ok {
			return fmt.Errorf("duplicate glossary source term %q", source)
		}
		seen[strings.ToLower(source)] = struct{}{}
	}
	return nil
}

func ParseGlossary(input string) ([]Glossary, error) {
	input = strings.TrimSpace(input)
	if input == "" {
		return nil, nil
	}
	if data, err := os.ReadFile(input); err == nil {
		return parseGlossaryJSON(data)
	}
	if strings.HasPrefix(input, "{") && strings.HasSuffix(input, "}") {
		return parseGlossaryJSON([]byte(input))
	}
	var result []Glossary
	for _, item := range strings.Split(strings.ReplaceAll(input, "，", ","), ",") {
		item = strings.TrimSpace(item)
		if item == "" {
			continue
		}
		found := false
		for _, separator := range []string{"->", "→", ":", "："} {
			if source, target, ok := strings.Cut(item, separator); ok {
				result = append(result, Glossary{Source: strings.TrimSpace(source), Target: strings.TrimSpace(target)})
				found = true
				break
			}
		}
		if !found {
			return nil, fmt.Errorf("invalid glossary item %q", item)
		}
	}
	return result, ValidateGlossary(result)
}

func parseGlossaryJSON(data []byte) ([]Glossary, error) {
	var values map[string]string
	if err := json.Unmarshal(data, &values); err != nil {
		return nil, fmt.Errorf("invalid glossary JSON: %w", err)
	}
	result := make([]Glossary, 0, len(values))
	for source, target := range values {
		result = append(result, Glossary{Source: source, Target: target})
	}
	return result, ValidateGlossary(result)
}

func BuildPrompt(text, target, source, instruction string, glossary []Glossary) string {
	targetName := languageNames[strings.ToLower(target)]
	if targetName == "" {
		targetName = target
	}
	sourceName := ""
	if source != "" && strings.ToLower(source) != "auto" {
		sourceName = languageNames[strings.ToLower(source)]
		if sourceName == "" {
			sourceName = source
		}
	}
	var constraints []string
	if len(glossary) > 0 {
		terms := make([]string, 0, len(glossary))
		for _, term := range glossary {
			terms = append(terms, strings.TrimSpace(term.Source)+"→"+strings.TrimSpace(term.Target))
		}
		constraints = append(constraints, "【硬性要求】专名/术语对照: "+strings.Join(terms, "、"))
	}
	for _, line := range strings.Split(strings.TrimSpace(instruction), "\n") {
		line = strings.TrimSpace(strings.TrimLeft(line, "0123456789. "))
		if line == "" {
			continue
		}
		if !strings.HasPrefix(line, "【") {
			line = "【注意】" + line
		}
		constraints = append(constraints, line)
	}
	if len(constraints) == 0 {
		if sourceName != "" {
			return fmt.Sprintf("请将以下%s文本翻译为%s，直接输出翻译结果，不要进行任何解释。\n\n%s", sourceName, targetName, strings.TrimSpace(text))
		}
		return fmt.Sprintf("请将以下文本翻译为%s，直接输出翻译结果，不要进行任何解释。\n\n%s", targetName, strings.TrimSpace(text))
	}
	header := "文本"
	if sourceName != "" {
		header = sourceName + header
	}
	lines := make([]string, len(constraints))
	for i, constraint := range constraints {
		lines[i] = fmt.Sprintf("%d. %s", i+1, constraint)
	}
	return fmt.Sprintf("请将以下%s翻译成%s，并且严格遵循所有约束要求。\n\n【源文】\n%s\n\n【约束要求】\n%s\n\n只输出译文，不要有任何额外说明。",
		header, targetName, strings.TrimSpace(text), strings.Join(lines, "\n"))
}
