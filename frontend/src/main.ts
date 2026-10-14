import "./styles.css";

type Glossary = { source: string; target: string };
type Settings = {
  version: number;
  api_base: string;
  api_method: string;
  api_key: string;
  model: string;
  timeout_seconds: number;
  user_agent: string;
  source: string;
  target: string;
  stream: boolean;
  max_tokens: number;
  temperature: number;
  instruction: string;
  glossary: Glossary[];
  disable_thinking: boolean;
  tray_enabled: boolean;
  close_to_tray: boolean;
  start_hidden: boolean;
};
type Language = { code: string; name: string };
type Detection = { code: string; name: string; uncertain: boolean; confidence: number; target: string };
type TranslationEvent = {
  requestID: string;
  sequence: number;
  kind: "started" | "delta" | "done" | "cancelled" | "error";
  text?: string;
  status?: string;
  error?: string;
};

const fallbackLanguages: Language[] = [
  { code: "en", name: "English" }, { code: "zh", name: "Chinese" },
  { code: "de", name: "German" }, { code: "fr", name: "French" },
  { code: "es", name: "Spanish" }, { code: "ja", name: "Japanese" },
  { code: "ko", name: "Korean" }, { code: "pt", name: "Portuguese" },
  { code: "ru", name: "Russian" }, { code: "ar", name: "Arabic" },
  { code: "it", name: "Italian" }, { code: "nl", name: "Dutch" },
  { code: "pl", name: "Polish" }, { code: "ro", name: "Romanian" },
  { code: "sv", name: "Swedish" }, { code: "tr", name: "Turkish" },
  { code: "hi", name: "Hindi" }, { code: "vi", name: "Vietnamese" },
  { code: "th", name: "Thai" }, { code: "id", name: "Indonesian" },
  { code: "ukr_cyrl", name: "Ukrainian" }, { code: "fas_arab", name: "Persian" },
  { code: "ces_latn", name: "Czech" }, { code: "ell_grek", name: "Greek" },
  { code: "dan_latn", name: "Danish" }, { code: "hun_latn", name: "Hungarian" },
  { code: "fin_latn", name: "Finnish" }, { code: "nob_latn", name: "Norwegian" },
  { code: "slk_latn", name: "Slovak" }, { code: "bul_cyrl", name: "Bulgarian" },
];

declare global {
  interface Window {
    go?: { main?: { App?: Record<string, (...args: unknown[]) => Promise<unknown>> } };
    runtime?: {
      EventsOn?: (name: string, handler: (payload: any) => void) => void;
    };
  }
}

const defaults: Settings = {
  version: 1,
  api_base: "https://index-translate.bilibili.com/v1",
  api_method: "chat/completions",
  api_key: "",
  model: "Index-Translate-35B-A3B",
  timeout_seconds: 120,
  user_agent: "yalt/dev",
  source: "auto",
  target: "auto",
  stream: true,
  max_tokens: 1024,
  temperature: 0,
  instruction: "",
  glossary: [],
  disable_thinking: true,
  tray_enabled: true,
  close_to_tray: true,
  start_hidden: false,
};

const root = document.querySelector<HTMLDivElement>("#app")!;
const api = () => window.go?.main?.App;
async function call<T>(method: string, ...args: unknown[]): Promise<T> {
  const fn = api()?.[method];
  if (!fn) throw new Error("The desktop bridge is not ready.");
  return await fn(...args) as T;
}

let settings = clone(defaults);
let draft = clone(defaults);
let languages = fallbackLanguages;
let selectedSource = "auto";
let autoDetectedSource = "";
let selectedTarget = "auto";
let text = "";
let output = "";
let requestID = "";
let lastSequence = 0;
let busy = false;
let composing = false;
let activeTab: "translate" | "settings" = "translate";
let detectionTimer = 0;
let copyFeedbackTimer = 0;

function clone(value: Settings): Settings {
  return { ...value, glossary: value.glossary.map((term) => ({ ...term })) };
}

function esc(value: string): string {
  return value.replace(/[&<>"']/g, (char) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#039;",
  })[char] ?? char);
}

function drawShell() {
  root.innerHTML = `
    <main class="app-frame">
      <header class="app-header" role="banner">
        <nav class="tabs segmented-control" aria-label="Application sections" role="tablist">
          <button class="tab active" id="tab-translate" type="button" role="tab" aria-selected="true" data-tab="translate">Translate</button>
          <button class="tab" id="tab-settings" type="button" role="tab" aria-selected="false" data-tab="settings">Settings</button>
        </nav>
      </header>
      <section class="view translate-view" id="view-translate" role="tabpanel" aria-labelledby="tab-translate">
        <div class="language-bar">
          <label class="language-select"><span>From</span><select id="source"></select></label>
          <span class="language-arrow" aria-hidden="true">→</span>
          <label class="language-select"><span>To</span><select id="target"></select></label>
        </div>
        <div class="editor-card">
          <div class="editor-heading"><span>Your text</span><span class="shortcut">Ctrl / ⌘ Return</span></div>
          <textarea id="input" aria-label="Text to translate" placeholder="Write or paste something…" spellcheck="true"></textarea>
          <div class="editor-actions">
            <span id="char-count">0 characters</span>
            <button type="button" class="plain-action" id="clear">Clear</button>
            <button type="button" class="translate-button" id="translate">Translate <span>↗</span></button>
          </div>
        </div>
        <div class="result-card">
          <div class="result-heading"><span>Translation</span><span id="result-status" role="status">Ready</span></div>
          <div class="result-text" id="output" aria-live="off"><span class="empty-result">Your translation will appear here.</span></div>
          <div class="result-actions">
            <span id="incomplete" class="incomplete" hidden>Incomplete result</span>
            <span id="copy-feedback" class="copy-feedback" role="status" aria-live="polite" hidden></span>
            <button type="button" class="plain-action" id="stop" hidden>Stop</button>
            <button type="button" class="plain-action copy-action" id="copy" disabled>Copy result</button>
          </div>
        </div>
      </section>
      <section class="view settings-view" id="view-settings" role="tabpanel" aria-labelledby="tab-settings" hidden>
        <div class="settings-intro"><h1>Settings</h1></div>
        <form id="settings-form" novalidate>
          <section class="settings-group">
            <h2 class="group-title">Connection</h2>
            <label class="setting-field">API base URL<input name="api_base" autocomplete="url" spellcheck="false"></label>
            <label class="setting-field">API method<select name="api_method">
              <option value="chat/completions">Chat Completions</option>
              <option value="responses">OpenAI Responses</option>
            </select></label>
            <label class="setting-field">API key (optional)<input name="api_key" type="password" autocomplete="new-password" spellcheck="false"></label>
            <label class="setting-field">Model<input name="model" spellcheck="false"></label>
            <label class="setting-field">Request deadline <small>1–600 seconds</small><input name="timeout_seconds" type="number" min="1" max="600"></label>
            <p class="endpoint-preview">Request URL <span id="endpoint"></span></p>
          </section>
          <section class="settings-group">
            <h2 class="group-title">Translation</h2>
            <div class="setting-columns">
              <label class="setting-field">Maximum output tokens<input name="max_tokens" type="number" min="1"></label>
              <label class="setting-field">Temperature<input name="temperature" type="number" min="0" step="any"></label>
            </div>
            <label class="setting-field">Extra instructions<textarea name="instruction" rows="3" placeholder="Optional style or formatting rules"></textarea></label>
            <div class="glossary-heading"><span>Glossary</span><button type="button" class="plain-action" id="add-term">+ Add term</button></div>
            <div id="glossary"></div>
            <label class="import-label">Import glossary JSON<input id="import-glossary" type="file" accept="application/json"></label>
            <div class="check-grid">
              <label><input name="stream" type="checkbox"> Stream output</label>
              <label><input name="disable_thinking" type="checkbox"> Hide reasoning text</label>
            </div>
          </section>
          <section class="settings-group">
            <h2 class="group-title">Desktop</h2>
            <div class="check-grid desktop-checks">
              <label><input name="tray_enabled" type="checkbox"> Enable tray icon <small>Restart required</small></label>
              <label><input name="close_to_tray" type="checkbox"> Close window to tray</label>
              <label><input name="start_hidden" type="checkbox"> Start hidden <small>Restart required</small></label>
            </div>
          </section>
          <div class="settings-bottom">
            <span id="settings-feedback" class="settings-feedback" role="status" aria-live="polite" hidden></span>
            <button type="button" class="plain-action" id="restore">Restore defaults</button>
            <button type="button" class="plain-action" id="cancel">Cancel</button>
            <button class="translate-button save-button" type="submit">Save settings <span>↗</span></button>
          </div>
        </form>
      </section>
    </main>
    <dialog class="confirm-dialog" id="confirm-dialog" aria-labelledby="confirm-title">
      <h2 id="confirm-title">Restore defaults?</h2>
      <p>This replaces the settings in the form. Save settings to apply them.</p>
      <div class="dialog-actions">
        <button type="button" class="plain-action" data-confirm="false">Cancel</button>
        <button type="button" class="translate-button" data-confirm="true">Restore defaults</button>
      </div>
    </dialog>`;

  fillLanguageSelects();
  setupTabs();
  setupTranslate();
  setupSettings();
  syncSettingsForm();
}

function fillLanguageSelects() {
  const languageOptions = languages.map((language) =>
    `<option value="${esc(language.code)}">${esc(language.name)}</option>`).join("");
  const source = document.querySelector<HTMLSelectElement>("#source")!;
  const target = document.querySelector<HTMLSelectElement>("#target")!;
  source.innerHTML = `<option value="auto">Detect language</option>${languageOptions}`;
  target.innerHTML = languageOptions;
  renderSourceSelection();
  renderTargetSelection();
}

function renderSourceSelection() {
  const source = document.querySelector<HTMLSelectElement>("#source");
  if (!source) return;
  source.value = selectedSource === "auto" ? autoDetectedSource || "auto" : selectedSource;
}

function renderTargetSelection() {
  const target = document.querySelector<HTMLSelectElement>("#target");
  if (!target) return;
  const sourceLanguage = selectedSource === "auto" ? autoDetectedSource : selectedSource;
  const automaticTarget = sourceLanguage === "zh" ? "en" : "zh";
  target.value = selectedTarget === "auto" ? automaticTarget : selectedTarget;
}

function setupTabs() {
  root.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((tab) => {
    tab.addEventListener("click", () => switchTab(tab.dataset.tab as "translate" | "settings"));
  });
}

function switchTab(tab: "translate" | "settings") {
  activeTab = tab;
  document.querySelector("#view-translate")!.toggleAttribute("hidden", tab !== "translate");
  document.querySelector("#view-settings")!.toggleAttribute("hidden", tab !== "settings");
  for (const name of ["translate", "settings"] as const) {
    const button = document.querySelector<HTMLButtonElement>(`#tab-${name}`)!;
    button.classList.toggle("active", tab === name);
    button.setAttribute("aria-selected", String(tab === name));
  }
  if (tab === "settings") {
    draft = clone(settings);
    syncSettingsForm();
  }
}

function setupTranslate() {
  const input = document.querySelector<HTMLTextAreaElement>("#input")!;
  input.value = text;
  input.addEventListener("compositionstart", () => { composing = true; });
  input.addEventListener("compositionend", () => {
    composing = false;
    text = input.value;
    onTextChanged();
  });
  input.addEventListener("input", () => {
    text = input.value;
    onTextChanged();
  });
  document.querySelector<HTMLSelectElement>("#source")!.addEventListener("change", (event) => {
    const choice = (event.currentTarget as HTMLSelectElement).value;
    autoDetectedSource = "";
    selectedSource = choice;
    renderSourceSelection();
    renderTargetSelection();
    window.clearTimeout(detectionTimer);
    if (selectedSource === "auto") {
      detectionTimer = window.setTimeout(() => { void detectLanguage(); }, 250);
    }
  });
  document.querySelector<HTMLSelectElement>("#target")!.addEventListener("change", (event) => {
    selectedTarget = (event.currentTarget as HTMLSelectElement).value;
  });
  document.querySelector("#clear")!.addEventListener("click", () => {
    text = "";
    input.value = "";
    output = "";
    updateOutput();
    onTextChanged();
  });
  document.querySelector("#translate")!.addEventListener("click", () => { void startTranslation(); });
  document.querySelector("#stop")!.addEventListener("click", () => { if (requestID) void call("CancelTranslation", requestID); });
  document.querySelector("#copy")!.addEventListener("click", () => {
    void copyOutput();
  });
}

function onTextChanged() {
  document.querySelector("#char-count")!.textContent = `${text.length.toLocaleString()} characters`;
  const button = document.querySelector<HTMLButtonElement>("#translate")!;
  button.disabled = busy || !text.trim();
  if (!busy) {
    output = "";
    updateOutput();
    setStatus("Ready");
    document.querySelector<HTMLElement>("#incomplete")!.hidden = true;
  }
  if (!text.trim()) {
    autoDetectedSource = "";
  }
  renderSourceSelection();
  renderTargetSelection();
  window.clearTimeout(detectionTimer);
  detectionTimer = window.setTimeout(() => { void detectLanguage(); }, 250);
}

async function detectLanguage() {
  if (!text.trim()) {
    autoDetectedSource = "";
    renderSourceSelection();
    renderTargetSelection();
    return;
  }
  if (composing || selectedSource !== "auto") {
    return;
  }
  const snapshot = text;
  try {
    const detected = await call<Detection>("DetectLanguage", snapshot);
    if (snapshot !== text || selectedSource !== "auto") return;
    if (!detected.uncertain && detected.code) {
      autoDetectedSource = detected.code;
    }
    renderSourceSelection();
    renderTargetSelection();
  } catch {
    // Keep the last detected language selected while detection is unavailable.
  }
}

async function copyOutput() {
  if (!output) return;
  let message: string;
  let result: "success" | "failure";
  try {
    await navigator.clipboard.writeText(output);
    message = "Copied to clipboard";
    result = "success";
  } catch {
    message = "Could not copy. Check clipboard access.";
    result = "failure";
  }
  const feedback = document.querySelector<HTMLElement>("#copy-feedback")!;
  feedback.textContent = message;
  feedback.classList.toggle("failure", result === "failure");
  feedback.hidden = false;
  window.clearTimeout(copyFeedbackTimer);
  copyFeedbackTimer = window.setTimeout(() => { feedback.hidden = true; }, 5000);
}

function updateOutput() {
  const content = document.querySelector<HTMLDivElement>("#output")!;
  content.textContent = output;
  if (!output) {
    content.innerHTML = '<span class="empty-result">Your translation will appear here.</span>';
  }
  document.querySelector<HTMLButtonElement>("#copy")!.disabled = !output;
}

async function startTranslation() {
  if (busy || !text.trim()) return;
  requestID = crypto.randomUUID();
  lastSequence = 0;
  output = "";
  busy = true;
  updateOutput();
  document.querySelector<HTMLElement>("#incomplete")!.hidden = true;
  setBusyUI(true);
  setStatus("Translating…");
  const requestText = text;
  try {
    await call("StartTranslation", {
      requestID,
      text: requestText,
      source: selectedSource === "auto" && autoDetectedSource ? autoDetectedSource : selectedSource,
      target: selectedTarget,
    });
  } catch (error) {
    busy = false;
    setBusyUI(false);
    setStatus(String(error), true);
  }
}

function setBusyUI(value: boolean) {
  busy = value;
  document.querySelector<HTMLButtonElement>("#translate")!.disabled = value || !text.trim();
  document.querySelector("#translate")!.innerHTML = value ? "Translating <span class=\"pulse\"></span>" : 'Translate <span>↗</span>';
  document.querySelector<HTMLButtonElement>("#stop")!.hidden = !value;
}

function setStatus(value: string, error = false) {
  const status = document.querySelector("#result-status")!;
  status.textContent = value;
  status.classList.toggle("error", error);
}

function receiveTranslation(event: TranslationEvent) {
  if (event.requestID !== requestID || event.sequence <= lastSequence) return;
  lastSequence = event.sequence;
  if (event.kind === "started") {
    setStatus(event.status ?? "Translating…");
    return;
  }
  if (event.kind === "delta") {
    output += event.text ?? "";
    updateOutput();
    return;
  }
  if (event.kind === "done" || event.kind === "error" || event.kind === "cancelled") {
    output = event.text ?? output;
    updateOutput();
    setBusyUI(false);
    setStatus(event.status ?? "Finished", event.kind === "error");
    document.querySelector<HTMLElement>("#incomplete")!.hidden = event.kind === "done";
  }
}

function setupSettings() {
  const form = document.querySelector<HTMLFormElement>("#settings-form")!;
  const updateEndpoint = () => {
    const base = (form.elements.namedItem("api_base") as HTMLInputElement).value;
    const method = (form.elements.namedItem("api_method") as HTMLSelectElement).value;
    (form.elements.namedItem("max_tokens") as HTMLInputElement).min = method === "responses" ? "16" : "1";
    document.querySelector("#endpoint")!.textContent = `${base.replace(/\/+$/, "")}/${method}`;
    showSettingsFeedback("");
  };
  form.addEventListener("input", updateEndpoint);
  form.addEventListener("change", updateEndpoint);
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    void saveSettings(form);
  });
  document.querySelector("#add-term")!.addEventListener("click", () => {
    readGlossaryForm();
    draft.glossary.push({ source: "", target: "" });
    drawGlossary();
  });
  document.querySelector("#glossary")!.addEventListener("click", (event) => {
    const button = (event.target as HTMLElement).closest<HTMLButtonElement>("[data-remove]");
    if (!button) return;
    readGlossaryForm();
    draft.glossary.splice(Number(button.dataset.remove), 1);
    drawGlossary();
  });
  document.querySelector<HTMLInputElement>("#import-glossary")!.addEventListener("change", (event) => {
    void importGlossary((event.currentTarget as HTMLInputElement).files?.[0]);
  });
  document.querySelector("#restore")!.addEventListener("click", () => { void restoreDefaults(); });
  document.querySelector("#cancel")!.addEventListener("click", () => {
    draft = clone(settings);
    switchTab("translate");
  });
}

function askConfirm(): Promise<boolean> {
  const dialog = document.querySelector<HTMLDialogElement>("#confirm-dialog")!;
  const buttons = dialog.querySelectorAll<HTMLButtonElement>("[data-confirm]");
  return new Promise((resolve) => {
    const finish = (value: boolean) => {
      buttons.forEach((button) => button.removeEventListener("click", onClick));
      dialog.removeEventListener("close", onClose);
      dialog.close();
      resolve(value);
    };
    const onClick = (event: Event) => {
      finish((event.currentTarget as HTMLElement).dataset.confirm === "true");
    };
    const onClose = () => { finish(false); };
    buttons.forEach((button) => button.addEventListener("click", onClick));
    dialog.addEventListener("close", onClose);
    dialog.showModal();
  });
}

async function restoreDefaults() {
  if (!await askConfirm()) return;
  draft = clone(defaults);
  syncSettingsForm();
  showSettingsFeedback("Defaults restored. Save settings to apply them.");
}

function syncSettingsForm() {
  const form = document.querySelector<HTMLFormElement>("#settings-form");
  if (!form) return;
  (form.elements.namedItem("api_method") as HTMLSelectElement).value = draft.api_method || defaults.api_method;
  for (const key of ["api_base", "api_key", "model", "timeout_seconds", "max_tokens", "temperature", "instruction"] as const) {
    (form.elements.namedItem(key) as HTMLInputElement | HTMLTextAreaElement).value = String(draft[key]);
  }
  for (const key of ["stream", "disable_thinking", "tray_enabled", "close_to_tray", "start_hidden"] as const) {
    (form.elements.namedItem(key) as HTMLInputElement).checked = draft[key];
  }
  document.querySelector("#endpoint")!.textContent = `${draft.api_base.replace(/\/+$/, "")}/${draft.api_method || defaults.api_method}`;
  drawGlossary();
}

function drawGlossary() {
  const list = document.querySelector<HTMLDivElement>("#glossary")!;
  if (!draft.glossary.length) {
    list.innerHTML = '<p class="empty-glossary">No terms added.</p>';
    return;
  }
  list.innerHTML = draft.glossary.map((term, index) => `
    <div class="term-row">
      <input name="glossary-source" value="${esc(term.source)}" placeholder="Source term" aria-label="Source term ${index + 1}">
      <span>→</span>
      <input name="glossary-target" value="${esc(term.target)}" placeholder="Translation" aria-label="Translation ${index + 1}">
      <button type="button" data-remove="${index}" aria-label="Remove glossary term">×</button>
    </div>`).join("");
  list.querySelectorAll<HTMLInputElement>("input").forEach((input) => {
    input.addEventListener("input", readGlossaryForm);
  });
}

function readGlossaryForm() {
  const sources = Array.from(document.querySelectorAll<HTMLInputElement>('[name="glossary-source"]'));
  const targets = Array.from(document.querySelectorAll<HTMLInputElement>('[name="glossary-target"]'));
  draft.glossary = sources.map((source, index) => ({ source: source.value, target: targets[index]?.value ?? "" }));
}

async function importGlossary(file?: File) {
  if (!file) return;
  try {
    const json: unknown = JSON.parse(await file.text());
    if (!json || typeof json !== "object" || Array.isArray(json)) throw new Error("Glossary must be a JSON object of source and target terms.");
    draft.glossary = Object.entries(json).map(([source, target]) => ({ source, target: String(target) }));
    drawGlossary();
    showSettingsFeedback("");
  } catch (error) {
    showSettingsFeedback(`Could not import glossary: ${String(error)}`, true);
  }
}

async function saveSettings(form: HTMLFormElement) {
  readGlossaryForm();
  const data = new FormData(form);
  draft.api_base = String(data.get("api_base") ?? "").trim();
  draft.api_method = String(data.get("api_method") ?? defaults.api_method);
  draft.api_key = String(data.get("api_key") ?? "").trim();
  draft.model = String(data.get("model") ?? "").trim();
  draft.timeout_seconds = Number(data.get("timeout_seconds"));
  draft.max_tokens = Number(data.get("max_tokens"));
  draft.temperature = Number(data.get("temperature"));
  draft.instruction = String(data.get("instruction") ?? "");
  for (const key of ["stream", "disable_thinking", "tray_enabled", "close_to_tray", "start_hidden"] as const) draft[key] = data.has(key);
  try {
    await call("SaveSettings", draft);
    settings = clone(draft);
    showSettingsFeedback("");
    switchTab("translate");
    setStatus("Settings saved");
  } catch (error) {
    showSettingsFeedback(String(error), true);
  }
}

function showSettingsFeedback(message: string, error = false) {
  const feedback = document.querySelector<HTMLElement>("#settings-feedback")!;
  feedback.textContent = message;
  feedback.classList.toggle("error", error);
  feedback.hidden = !message;
}

window.runtime?.EventsOn?.("translation", receiveTranslation);
window.runtime?.EventsOn?.("navigate", (value: string) => {
  if (value === "settings") switchTab("settings");
});

drawShell();
Promise.all([
  call<Settings>("GetSettings").catch(() => clone(defaults)),
  call<Language[]>("GetLanguages").catch(() => []),
]).then(([savedSettings, availableLanguages]) => {
  settings = clone(savedSettings);
  draft = clone(settings);
  selectedSource = settings.source || "auto";
  selectedTarget = settings.target || "auto";
  languages = availableLanguages?.length ? availableLanguages : fallbackLanguages;
  fillLanguageSelects();
  syncSettingsForm();
});

document.addEventListener("keydown", (event) => {
  const target = event.target;
  if (activeTab !== "translate" || !(target instanceof HTMLTextAreaElement) ||
      target.id !== "input" || event.isComposing || composing || event.keyCode === 229) return;
  if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
    event.preventDefault();
    void startTranslation();
  }
});
