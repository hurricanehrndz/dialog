(() => {
  // ui-src/main.ts
  var invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
  var el = (tag, className, text) => {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== void 0) node.textContent = text;
    return node;
  };
  function applyFontSpec(node, spec) {
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
  function sendEvent(event) {
    void invoke("ui_event", { event });
  }
  function render(state) {
    document.body.classList.add(`platform-${state.platform}`);
    if (state.window.appearance) {
      document.documentElement.dataset.appearance = state.window.appearance;
    }
    const root = el("div", "dialog");
    if (state.title !== null) {
      const header = el("div", "header");
      const title = el("h1", "title", state.title);
      applyFontSpec(title, state.titleFont);
      header.appendChild(title);
      if (state.subtitle) header.appendChild(el("h2", "subtitle", state.subtitle));
      if (state.window.moveable) header.setAttribute("data-tauri-drag-region", "");
      root.appendChild(header);
    }
    const main = el("div", "main-row");
    if (!state.icon.hidden) {
      const iconBox = el("div", "icon");
      iconBox.style.width = `${state.icon.size}px`;
      iconBox.style.opacity = String(state.icon.alpha);
      iconBox.appendChild(el("div", "icon-default", "\u{1F4AC}"));
      iconBox.setAttribute("role", "img");
      iconBox.setAttribute("aria-label", state.icon.altText);
      main.appendChild(iconBox);
    }
    const message = el("div", "message", state.message);
    message.style.textAlign = state.messageAlignment;
    if (state.messagePosition === "centre" || state.messagePosition === "center") {
      message.classList.add("v-center");
    } else if (state.messagePosition === "bottom") {
      message.classList.add("v-bottom");
    }
    applyFontSpec(message, state.messageFont);
    main.appendChild(message);
    root.appendChild(main);
    const buttons = el("div", "buttons");
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
      const total = state.timer.seconds * 1e3;
      const start = performance.now();
      const tick = (now) => {
        const remaining = Math.max(0, total - (now - start));
        fill.style.width = `${remaining / total * 100}%`;
        label.textContent = String(Math.ceil(remaining / 1e3));
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
    document.addEventListener("keydown", (e) => {
      if (e.key === "Enter" && state.button1.enabled) sendEvent("button1");
      if (e.key === "Escape" && state.button2.visible && state.button2.enabled)
        sendEvent("button2");
    });
  }
  invoke("get_state").then((state) => render(state));
})();
