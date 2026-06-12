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
  main.appendChild(message);
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
  b1.addEventListener("click", () => sendEvent("button1"));
  buttons.appendChild(b1);
  root.appendChild(buttons);

  document.body.replaceChildren(root);
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
  if (e.key === "Enter" && current.button1.enabled) sendEvent("button1");
  if (e.key === "Escape" && current.button2.visible && current.button2.enabled)
    sendEvent("button2");
});

invoke("get_state").then((state) => show(state as DialogState));
// Live updates from the command-file watcher: full-state push per change.
void window.__TAURI__.event.listen("state", (e) => show(e.payload as DialogState));
