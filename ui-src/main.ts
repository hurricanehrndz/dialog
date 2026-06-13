// Dialog renderer: draws one DialogState and emits semantic UI events.
// All contract logic lives on the Rust side (design D2); this file only
// renders state and reports interactions.

import { marked } from "marked";

interface ButtonState {
  text: string;
  visible: boolean;
  enabled: boolean;
  action: string | null;
}

interface ListItem {
  title: string;
  subtitle: string | null;
  icon: string | null;
  status: string;
  statusText: string;
  progress: number | null;
  selected: boolean;
}

interface TextField {
  name: string;
  title: string;
  value: string;
  prompt: string | null;
  secure: boolean;
  required: boolean;
  regex: string | null;
  regexError: string | null;
  isDate: boolean;
}

interface Checkbox {
  name: string;
  label: string;
  checked: boolean;
  disabled: boolean;
  style: string;
}

interface Select {
  name: string;
  title: string;
  values: string[];
  selected: string;
  required: boolean;
  style: string;
}

interface DialogState {
  platform: "macos" | "windows" | "linux";
  title: string | null;
  subtitle: string | null;
  message: string;
  messageAlignment: "left" | "center" | "right";
  messagePosition: string | null;
  titleFont: string | null;
  messageFont: string | null;
  icon: {
    source: string;
    size: number;
    alpha: number;
    altText: string;
    overlay: string | null;
    hidden: boolean;
  };
  button1: ButtonState;
  button2: ButtonState;
  infoButton: ButtonState;
  timer: { seconds: number; hideBar: boolean } | null;
  listItems: ListItem[];
  listSelectEnabled: boolean;
  listStyle: string | null;
  textFields: TextField[];
  checkboxes: Checkbox[];
  selects: Select[];
  progress: { total: number; current: number | null; text: string; visible: boolean } | null;
  infoText: string | null;
  window: { appearance: string | null; moveable: boolean };
  mini: boolean;
  style: string | null;
  quitKey: string;
}

declare global {
  interface Window {
    __TAURI__: {
      core: { invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> };
      event: {
        listen: (name: string, handler: (e: { payload: unknown }) => void) => Promise<unknown>;
      };
    };
  }
}

const invoke = (cmd: string, args?: Record<string, unknown>) =>
  window.__TAURI__.core.invoke(cmd, args);

const el = <K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className?: string,
  text?: string,
): HTMLElementTagNameMap[K] => {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
};

/// Parse swiftDialog font specs ("size=30,colour=#FF0000,weight=bold").
function applyFontSpec(node: HTMLElement, spec: string | null) {
  if (!spec) return;
  for (const part of spec.split(",")) {
    const [key, value] = part.split("=").map((s) => s.trim());
    if (!value) continue;
    if (key === "size") node.style.fontSize = `${parseFloat(value)}px`;
    if (key === "colour" || key === "color") node.style.color = value;
    if (key === "weight") node.style.fontWeight = value;
    if (key === "name") node.style.fontFamily = value;
  }
}

function sendEvent(event: string) {
  void invoke("ui_event", { event });
}

const STATUS_GLYPHS: Record<string, string> = {
  success: "✓",
  fail: "✕",
  error: "✕",
  pending: "•••",
};

function renderList(state: DialogState): HTMLElement {
  const list = el("div", "list");
  if (state.listStyle === "compact") list.classList.add("compact");
  state.listItems.forEach((item, index) => {
    const row = el("div", "list-row");
    if (state.listSelectEnabled) {
      const box = el("input") as HTMLInputElement;
      box.type = "checkbox";
      box.checked = item.selected;
      box.addEventListener("change", () =>
        invoke("list_select", { index, selected: box.checked }),
      );
      row.appendChild(box);
    }
    const text = el("div", "list-text");
    text.appendChild(el("div", "list-title", item.title));
    if (item.subtitle) text.appendChild(el("div", "list-subtitle", item.subtitle));
    row.appendChild(text);
    if (item.statusText) row.appendChild(el("div", "list-statustext", item.statusText));
    const status = el("div", `list-status status-${item.status || "none"}`);
    if (item.status === "wait") {
      status.appendChild(el("div", "spinner"));
    } else if (item.status === "progress") {
      const bar = el("div", "row-progress");
      const fill = el("div", "row-progress-fill");
      fill.style.width = `${item.progress ?? 0}%`;
      bar.appendChild(fill);
      status.appendChild(bar);
    } else if (STATUS_GLYPHS[item.status]) {
      status.textContent = STATUS_GLYPHS[item.status];
    }
    row.appendChild(status);
    list.appendChild(row);
  });
  return list;
}

function render(state: DialogState) {
  document.body.classList.add(`platform-${state.platform}`);
  if (state.window.appearance) {
    document.documentElement.dataset.appearance = state.window.appearance;
  }
  // Chromeless window: --moveable turns the whole surface into a drag
  // region (buttons/inputs stay interactive — Tauri ignores drags that
  // start on interactive elements only if marked, so scope to header/row).
  const root = el("div", "dialog");
  if (state.mini) root.classList.add("mini");

  // Title row: centered across the full window width (upstream layout).
  if (state.title !== null) {
    const header = el("div", "header");
    const title = el("h1", "title", state.title);
    applyFontSpec(title, state.titleFont);
    header.appendChild(title);
    if (state.subtitle) header.appendChild(el("h2", "subtitle", state.subtitle));
    if (state.window.moveable) header.setAttribute("data-tauri-drag-region", "");
    root.appendChild(header);
  }

  // Main row: icon beside message.
  const main = el("div", "main-row");
  if (!state.icon.hidden) {
    const iconBox = el("div", "icon");
    iconBox.style.width = `${state.icon.size}px`;
    iconBox.style.opacity = String(state.icon.alpha);
    // Placeholder glyph until the icon-resolution pipeline lands (group 5).
    iconBox.appendChild(el("div", "icon-default", "💬"));
    iconBox.setAttribute("role", "img");
    iconBox.setAttribute("aria-label", state.icon.altText);
    main.appendChild(iconBox);
  }
  const message = el("div", "message");
  // CommonMark per the dialog-core spec; links are intercepted below and
  // opened in the default browser by the Rust side.
  message.innerHTML = marked.parse(state.message, { async: false });
  message.addEventListener("click", (e) => {
    const link = (e.target as HTMLElement).closest("a");
    if (link?.href) {
      e.preventDefault();
      void invoke("open_link", { url: link.href });
    }
  });
  message.style.textAlign = state.messageAlignment;
  if (state.messagePosition === "centre" || state.messagePosition === "center") {
    message.classList.add("v-center");
  } else if (state.messagePosition === "bottom") {
    message.classList.add("v-bottom");
  }
  applyFontSpec(message, state.messageFont);

  const hasInputs =
    state.textFields.length > 0 ||
    state.checkboxes.length > 0 ||
    state.selects.length > 0;
  if (state.listItems.length > 0 || hasInputs) {
    const content = el("div", "content-col");
    if (state.message.trim()) content.appendChild(message);
    if (hasInputs) content.appendChild(renderInputs(state));
    if (state.listItems.length > 0) content.appendChild(renderList(state));
    main.appendChild(content);
  } else {
    main.appendChild(message);
  }
  root.appendChild(main);

  // Overall progress bar (+ progresstext) above the button row.
  if (state.progress && state.progress.visible) {
    const wrap = el("div", "progress");
    const bar = el("div", "progress-bar");
    const fill = el("div", "progress-fill");
    if (state.progress.current === null) {
      fill.classList.add("indeterminate");
    } else {
      fill.style.width = `${(state.progress.current / state.progress.total) * 100}%`;
    }
    bar.appendChild(fill);
    wrap.appendChild(bar);
    if (state.progress.text.trim()) {
      wrap.appendChild(el("div", "progress-text", state.progress.text));
    }
    root.appendChild(wrap);
  }

  // Bottom bar: [infotext] [info] [timer bar] [button2] [button1].
  const buttons = el("div", "buttons");
  if (state.infoText) {
    buttons.appendChild(el("div", "infotext", state.infoText));
  }
  if (state.infoButton.visible) {
    const info = el("button", "btn info", state.infoButton.text);
    info.addEventListener("click", () => sendEvent("info"));
    buttons.appendChild(info);
  }
  if (state.timer && !state.timer.hideBar) {
    const bar = el("div", "timer-bar");
    const fill = el("div", "timer-fill");
    const label = el("div", "timer-label");
    bar.appendChild(fill);
    bar.appendChild(label);
    buttons.appendChild(bar);
    const total = state.timer.seconds * 1000;
    const start = performance.now();
    const tick = (now: number) => {
      const remaining = Math.max(0, total - (now - start));
      fill.style.width = `${(remaining / total) * 100}%`;
      label.textContent = String(Math.ceil(remaining / 1000));
      if (remaining > 0) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  } else {
    buttons.appendChild(el("div", "spacer"));
  }
  if (state.button2.visible) {
    const b2 = el("button", "btn", state.button2.text);
    b2.disabled = !state.button2.enabled;
    b2.addEventListener("click", () => sendEvent("button2"));
    buttons.appendChild(b2);
  }
  const b1 = el("button", "btn primary", state.button1.text);
  b1.disabled = !state.button1.enabled;
  b1.addEventListener("click", () => void invoke("submit"));
  buttons.appendChild(b1);
  root.appendChild(buttons);

  // Validation error sheet (hidden until "validation-errors" fires).
  const sheet = el("div", "error-sheet");
  sheet.id = "error-sheet";
  root.appendChild(sheet);

  document.body.replaceChildren(root);
}

function checkboxRow(cb: Checkbox): HTMLElement {
  const row = el("label", "checkbox-row");
  const input = el("input") as HTMLInputElement;
  input.type = "checkbox";
  input.checked = cb.checked;
  input.disabled = cb.disabled;
  if (cb.style === "switch") input.classList.add("switch");
  input.addEventListener("change", () =>
    invoke("set_checkbox", { name: cb.name, checked: input.checked }),
  );
  row.appendChild(input);
  row.appendChild(el("span", "checkbox-label", cb.label));
  return row;
}

function textFieldRow(tf: TextField): HTMLElement {
  const row = el("div", "input-row");
  row.appendChild(el("label", "input-label", tf.title));
  const input = el("input", "input-control") as HTMLInputElement;
  input.type = tf.secure ? "password" : "text";
  input.value = tf.value;
  if (tf.prompt) input.placeholder = tf.prompt;
  input.addEventListener("input", () =>
    invoke("set_field", { name: tf.name, value: input.value }),
  );
  row.appendChild(input);
  return row;
}

function selectRow(sel: Select): HTMLElement {
  const row = el("div", "input-row");
  row.appendChild(el("label", "input-label", sel.title));
  if (sel.style === "radio") {
    const group = el("div", "radio-group");
    for (const v of sel.values) {
      const opt = el("label", "radio-opt");
      const radio = el("input") as HTMLInputElement;
      radio.type = "radio";
      radio.name = `sel-${sel.name}`;
      radio.checked = v === sel.selected;
      radio.addEventListener("change", () =>
        invoke("set_select", { name: sel.name, value: v }),
      );
      opt.appendChild(radio);
      opt.appendChild(document.createTextNode(v));
      group.appendChild(opt);
    }
    row.appendChild(group);
  } else {
    const dropdown = el("select", "input-control") as HTMLSelectElement;
    if (!sel.selected) {
      const placeholder = el("option", undefined, "") as HTMLOptionElement;
      placeholder.value = "";
      dropdown.appendChild(placeholder);
    }
    for (const v of sel.values) {
      const opt = el("option", undefined, v) as HTMLOptionElement;
      opt.value = v;
      opt.selected = v === sel.selected;
      dropdown.appendChild(opt);
    }
    dropdown.addEventListener("change", () =>
      invoke("set_select", { name: sel.name, value: dropdown.value }),
    );
    row.appendChild(dropdown);
  }
  return row;
}

function renderInputs(state: DialogState): HTMLElement {
  const box = el("div", "inputs");
  // Upstream element order: checkboxes, then textfields, then selects
  // (verified vs swiftDialog 3.0.1 form layout).
  state.checkboxes.forEach((cb) => box.appendChild(checkboxRow(cb)));
  state.textFields.forEach((tf) => box.appendChild(textFieldRow(tf)));
  state.selects.forEach((sel) => box.appendChild(selectRow(sel)));
  return box;
}

function showValidationErrors(messages: string[]) {
  const sheet = document.getElementById("error-sheet");
  if (!sheet) return;
  if (messages.length === 0) {
    sheet.classList.remove("visible");
    return;
  }
  sheet.replaceChildren(
    ...messages.map((m) => el("div", "error-line", `• ${m}`)),
  );
  sheet.classList.add("visible");
}

let current: DialogState | null = null;

function show(state: DialogState) {
  current = state;
  render(state);
}

// Keyboard contract: Return = button1, Escape = button2 (when visible),
// Cmd/Ctrl+<quitkey> = exit 10. Bound once; reads the latest state.
document.addEventListener("keydown", (e) => {
  if (!current) return;
  if ((e.metaKey || e.ctrlKey) && e.key === current.quitKey) {
    sendEvent("quitkey");
    return;
  }
  if (e.key === "Enter" && current.button1.enabled) void invoke("submit");
  if (e.key === "Escape" && current.button2.visible && current.button2.enabled)
    sendEvent("button2");
});

invoke("get_state").then((state) => show(state as DialogState));
// Live updates from the command-file watcher: full-state push per change.
void window.__TAURI__.event.listen("state", (e) => show(e.payload as DialogState));
// Submit-time validation failures from the Rust side.
void window.__TAURI__.event.listen("validation-errors", (e) =>
  showValidationErrors(e.payload as string[]),
);
