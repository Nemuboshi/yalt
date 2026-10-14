package translation

import "testing"

func TestDetectLanguages(t *testing.T) {
	tests := []struct {
		name string
		text string
		code string
	}{
		{"simplified Chinese", "今天天气很好，我们一起去公园散步吧。", "zh"},
		{"traditional Chinese", "今天天氣很好，我們一起去公園散步吧。", "zh"},
		{"Japanese", "今日は天気が良いので、一緒に公園を散歩しましょう。", "ja"},
		{"English", "The weather is lovely today. Let us take a walk in the park.", "en"},
		{"French", "Le temps est magnifique aujourd'hui. Allons nous promener au parc.", "fr"},
		{"Ukrainian", "Сьогодні чудова погода. Давайте прогуляємося в парку.", "ukr_cyrl"},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			got := Detect(test.text)
			if got.Code != test.code {
				t.Fatalf("language = %q, want %q (%s)", got.Code, test.code, got.Name)
			}
			if got.Uncertain {
				t.Fatalf("detector marked %s as uncertain", test.name)
			}
			if got.Confidence <= 0 {
				t.Fatalf("confidence = %v, want a positive relative score", got.Confidence)
			}
		})
	}
}

func TestDetectEmptyText(t *testing.T) {
	got := Detect(" \t\n")
	if !got.Uncertain || got.Code != "" || got.Target != "zh" {
		t.Fatalf("empty detection = %#v", got)
	}
}

func TestSelectTargetHonoursManualTarget(t *testing.T) {
	got := SelectTarget("auto", "de", Detection{Code: "zh"})
	if got != "de" {
		t.Fatalf("target = %q, want de", got)
	}
}
