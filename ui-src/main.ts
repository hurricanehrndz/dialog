// Dialog renderer: draws one DialogState and emits semantic UI events.
// All contract logic lives on the Rust side (design D2); this file only
// renders state and reports interactions.

import { marked } from "marked";

interface ButtonState {
  text: string;
  visible: boolean;
  enabled: boolean;
  action: string | null;
  symbol: IconRender | null;
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

// Resolved icon representation from the Rust pipeline (see icon.rs).
type IconRender =
  | { kind: "none" }
  | { kind: "image"; url: string }
  | { kind: "glyph"; glyph: string; color: string | null }
  | { kind: "placeholder" };

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
    centered: boolean;
    overlay: string | null;
    hidden: boolean;
    render: IconRender;
    overlayRender: IconRender | null;
  };
  background: {
    source: string;
    render: IconRender;
    alpha: number;
    position: string | null;
    size: string;
  } | null;
  banner: {
    source: string;
    render: IconRender;
    title: string | null;
    height: number | null;
  } | null;
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
  window: {
    appearance: string | null;
    moveable: boolean;
    fullscreen: boolean;
    blur: boolean;
  };
  mini: boolean;
  style: string | null;
  quitKey: string;
  buttonStyle: string | null;
  buttonSize: string;
  buttonTextSize: number | null;
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

/// Draw a resolved icon: an image, a Fluent icon-font glyph, or — when the
/// source was recognised but not renderable here — the generic placeholder.
function drawIcon(render: IconRender): HTMLElement {
  if (render.kind === "image") {
    const img = el("img", "icon-img") as HTMLImageElement;
    img.src = render.url;
    img.decoding = "async";
    return img;
  }
  if (render.kind === "glyph") {
    const span = el("span", "icon-glyph", render.glyph);
    if (render.color) span.style.color = render.color;
    return span;
  }
  return el("div", "icon-default", "💬");
}

/// Build a button, prepending its symbol glyph/image (--button*symbol).
function buttonEl(cls: string, b: ButtonState, onClick: () => void): HTMLButtonElement {
  const btn = el("button", cls) as HTMLButtonElement;
  if (b.symbol && b.symbol.kind !== "none") {
    const sym = drawIcon(b.symbol);
    sym.classList.add("btn-symbol");
    btn.appendChild(sym);
  }
  btn.appendChild(document.createTextNode(b.text));
  btn.disabled = !b.enabled;
  btn.addEventListener("click", onClick);
  return btn;
}

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
  // --fullscreen: backdrop covers the display, dialog centered as a card.
  document.body.classList.toggle("fullscreen", state.window.fullscreen);
  // --blurscreen: transparent webview so the dimmed desktop shows through.
  document.body.classList.toggle("blurscreen", state.window.blur);
  document.documentElement.style.background = state.window.blur ? "transparent" : "";
  if (state.window.appearance) {
    document.documentElement.dataset.appearance = state.window.appearance;
  }
  // Chromeless window: --moveable turns the whole surface into a drag
  // region (buttons/inputs stay interactive — Tauri ignores drags that
  // start on interactive elements only if marked, so scope to header/row).
  const root = el("div", "dialog");
  if (state.mini) root.classList.add("mini");

  // Top banner (`--bannerimage`): spans the full width and carries the title.
  const bannerTitle = state.banner ? state.banner.title ?? state.title : null;
  if (state.banner) {
    const banner = el("div", "banner");
    if (state.banner.height) banner.style.height = `${state.banner.height}px`;
    if (state.banner.render.kind === "image") {
      banner.style.backgroundImage = `url("${state.banner.render.url}")`;
    }
    if (bannerTitle) banner.appendChild(el("div", "banner-title", bannerTitle));
    if (state.window.moveable) banner.setAttribute("data-tauri-drag-region", "");
    root.appendChild(banner);
  }

  // Title row: centered across the full window width (upstream layout). When a
  // banner is shown it carries the title, so the header only adds the subtitle.
  const showHeaderTitle = state.title !== null && !state.banner;
  if (showHeaderTitle || state.subtitle) {
    const header = el("div", "header");
    if (showHeaderTitle) {
      const title = el("h1", "title", state.title!);
      applyFontSpec(title, state.titleFont);
      header.appendChild(title);
    }
    if (state.subtitle) header.appendChild(el("h2", "subtitle", state.subtitle));
    if (state.window.moveable) header.setAttribute("data-tauri-drag-region", "");
    root.appendChild(header);
  }

  // Main row: icon beside message.
  const main = el("div", "main-row");
  if (!state.icon.hidden && state.icon.render.kind !== "none") {
    const iconBox = el("div", "icon");
    if (state.icon.centered) iconBox.classList.add("centered");
    iconBox.style.width = `${state.icon.size}px`;
    iconBox.style.height = `${state.icon.size}px`;
    // Box drives glyph size (font-size) and image bounds.
    iconBox.style.fontSize = `${state.icon.size}px`;
    iconBox.style.opacity = String(state.icon.alpha);
    iconBox.appendChild(drawIcon(state.icon.render));
    // Overlay badge over the lower-trailing corner (--overlayicon).
    if (state.icon.overlayRender && state.icon.overlayRender.kind !== "none") {
      const badge = el("div", "icon-overlay");
      badge.appendChild(drawIcon(state.icon.overlayRender));
      iconBox.appendChild(badge);
    }
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
  if (state.buttonStyle === "stack") buttons.classList.add("stack");
  else if (state.buttonStyle === "center" || state.buttonStyle === "centre")
    buttons.classList.add("center");
  buttons.classList.add(`size-${state.buttonSize}`);
  if (state.buttonTextSize) buttons.style.fontSize = `${state.buttonTextSize}px`;
  if (state.infoText) {
    buttons.appendChild(el("div", "infotext", state.infoText));
  }
  if (state.infoButton.visible) {
    buttons.appendChild(buttonEl("btn info", state.infoButton, () => sendEvent("info")));
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
    buttons.appendChild(buttonEl("btn", state.button2, () => sendEvent("button2")));
  }
  buttons.appendChild(buttonEl("btn primary", state.button1, () => void invoke("submit")));
  root.appendChild(buttons);

  // Validation error sheet (hidden until "validation-errors" fires).
  const sheet = el("div", "error-sheet");
  sheet.id = "error-sheet";
  root.appendChild(sheet);

  // Full-window background image (--background) behind all content.
  if (state.background && state.background.render.kind === "image") {
    const bg = el("div", "bg-image");
    bg.style.backgroundImage = `url("${state.background.render.url}")`;
    bg.style.opacity = String(state.background.alpha);
    bg.style.backgroundSize = state.background.size;
    if (state.background.position) bg.style.backgroundPosition = state.background.position;
    document.body.replaceChildren(bg, root);
  } else {
    document.body.replaceChildren(root);
  }
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
