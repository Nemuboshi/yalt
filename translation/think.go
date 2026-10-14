package translation

import (
	"errors"
	"strings"
)

type ThinkFilter struct {
	buffer      string
	inThinking  bool
	sawThinking bool
}

func (f *ThinkFilter) Feed(input string) (string, error) {
	f.buffer += input
	var out strings.Builder
	for {
		if f.inThinking {
			end := strings.Index(f.buffer, "</think>")
			if end < 0 {
				if len(f.buffer) > len("</think>") {
					f.buffer = f.buffer[len(f.buffer)-len("</think>")+1:]
				}
				return out.String(), nil
			}
			f.buffer = f.buffer[end+len("</think>"):]
			f.inThinking = false
			continue
		}
		start := strings.Index(f.buffer, "<think>")
		if start == 0 {
			f.sawThinking = true
			f.inThinking = true
			f.buffer = f.buffer[len("<think>"):]
			continue
		}
		if start > 0 {
			out.WriteString(f.buffer[:start])
			f.buffer = f.buffer[start:]
			continue
		}
		keep := len(f.buffer)
		for _, tag := range []string{"<think>", "</think>"} {
			for i := 1; i < len(tag) && i <= len(f.buffer); i++ {
				if strings.HasPrefix(tag[:i], f.buffer) {
					keep = min(keep, len(f.buffer)-i)
				}
			}
		}
		if keep == len(f.buffer) {
			out.WriteString(f.buffer)
			f.buffer = ""
			return out.String(), nil
		}
		if keep > 0 {
			out.WriteString(f.buffer[:keep])
			f.buffer = f.buffer[keep:]
		}
		return out.String(), nil
	}
}

func (f *ThinkFilter) Flush() (string, error) {
	if f.inThinking {
		return "", errors.New("response ended inside a reasoning block")
	}
	out := f.buffer
	f.buffer = ""
	return out, nil
}
