package translation

import (
	"strings"
	"sync"

	"github.com/pemistahl/lingua-go"
)

type Language struct {
	Code string `json:"code"`
	Name string `json:"name"`
}

var Languages = []Language{
	{Code: "en", Name: "English"}, {Code: "zh", Name: "Chinese"},
	{Code: "de", Name: "German"}, {Code: "fr", Name: "French"},
	{Code: "es", Name: "Spanish"}, {Code: "ja", Name: "Japanese"},
	{Code: "ko", Name: "Korean"}, {Code: "pt", Name: "Portuguese"},
	{Code: "ru", Name: "Russian"}, {Code: "ar", Name: "Arabic"},
	{Code: "it", Name: "Italian"}, {Code: "nl", Name: "Dutch"},
	{Code: "pl", Name: "Polish"}, {Code: "ro", Name: "Romanian"},
	{Code: "sv", Name: "Swedish"}, {Code: "tr", Name: "Turkish"},
	{Code: "hi", Name: "Hindi"}, {Code: "vi", Name: "Vietnamese"},
	{Code: "th", Name: "Thai"}, {Code: "id", Name: "Indonesian"},
	{Code: "ukr_cyrl", Name: "Ukrainian"}, {Code: "fas_arab", Name: "Persian"},
	{Code: "ces_latn", Name: "Czech"}, {Code: "ell_grek", Name: "Greek"},
	{Code: "dan_latn", Name: "Danish"}, {Code: "hun_latn", Name: "Hungarian"},
	{Code: "fin_latn", Name: "Finnish"}, {Code: "nob_latn", Name: "Norwegian"},
	{Code: "slk_latn", Name: "Slovak"}, {Code: "bul_cyrl", Name: "Bulgarian"},
}

var languageNames = map[string]string{
	"en": "英语", "zh": "中文", "de": "德语", "fr": "法语", "es": "西班牙语",
	"ja": "日语", "ko": "韩语", "pt": "葡萄牙语", "ru": "俄语", "ar": "阿拉伯语",
	"it": "意大利语", "nl": "荷兰语", "pl": "波兰语", "ro": "罗马尼亚语",
	"sv": "瑞典语", "tr": "土耳其语", "hi": "印地语", "vi": "越南语",
	"th": "泰语", "id": "印尼语", "ukr_cyrl": "乌克兰语", "fas_arab": "波斯语",
	"ces_latn": "捷克语", "ell_grek": "希腊语", "dan_latn": "丹麦语",
	"hun_latn": "匈牙利语", "fin_latn": "芬兰语", "nob_latn": "书面挪威语",
	"slk_latn": "斯洛伐克语", "bul_cyrl": "保加利亚语",
}

var (
	detectorOnce sync.Once
	detector     lingua.LanguageDetector
	detectorMu   sync.Mutex
)

type Detection struct {
	Code       string  `json:"code"`
	Name       string  `json:"name"`
	Uncertain  bool    `json:"uncertain"`
	Confidence float64 `json:"confidence"`
	Target     string  `json:"target"`
}

func Detect(text string) Detection {
	text = strings.TrimSpace(text)
	if text == "" {
		return Detection{Uncertain: true, Target: "zh"}
	}
	detectorOnce.Do(func() {
		detector = lingua.NewLanguageDetectorBuilder().
			FromAllSpokenLanguages().
			WithLowAccuracyMode().
			Build()
	})

	detectorMu.Lock()
	language, exists := detector.DetectLanguageOf(text)
	confidenceValues := detector.ComputeLanguageConfidenceValues(text)
	detectorMu.Unlock()

	if !exists || language == lingua.Unknown {
		return Detection{Code: "unknown", Name: "Unknown", Uncertain: true, Target: "zh"}
	}
	confidence := 0.0
	if len(confidenceValues) > 0 {
		confidence = confidenceValues[0].Value()
	}
	code := providerLanguageCode(strings.ToLower(language.IsoCode639_1().String()))
	return Detection{
		Code:       code,
		Name:       language.String(),
		Confidence: confidence,
		Target:     targetForLanguage(code),
	}
}

func providerLanguageCode(code string) string {
	switch code {
	case "uk":
		return "ukr_cyrl"
	case "fa":
		return "fas_arab"
	case "cs":
		return "ces_latn"
	case "el":
		return "ell_grek"
	case "da":
		return "dan_latn"
	case "hu":
		return "hun_latn"
	case "fi":
		return "fin_latn"
	case "nb":
		return "nob_latn"
	case "nn":
		return "nob_latn"
	case "sk":
		return "slk_latn"
	case "bg":
		return "bul_cyrl"
	case "tl":
		return "fil"
	default:
		return code
	}
}

func targetForLanguage(code string) string {
	if code == "zh" {
		return "en"
	}
	return "zh"
}

func SelectTarget(source, target string, detection Detection) string {
	if strings.ToLower(target) != "" && strings.ToLower(target) != "auto" {
		return target
	}
	if strings.ToLower(source) != "" && strings.ToLower(source) != "auto" {
		if strings.ToLower(source) == "zh" {
			return "en"
		}
		return "zh"
	}
	return targetForLanguage(detection.Code)
}

func LanguageName(code string) string {
	if name, ok := languageNames[strings.ToLower(code)]; ok {
		return name
	}
	return code
}
