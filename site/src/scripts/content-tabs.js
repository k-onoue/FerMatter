(() => {
  const navigation = document.querySelector("[data-content-tabs]");
  if (!navigation) return;
  const entries = Array.from(navigation.querySelectorAll('a[href^="#"]'))
    .map((tab) => ({
      tab,
      panel: document.getElementById(tab.getAttribute("href").slice(1)),
    }))
    .filter(({ panel }) => panel);
  if (entries.length < 2) return;

  function entryFromHash() {
    let id;
    try {
      id = decodeURIComponent(window.location.hash.slice(1));
    } catch {
      return;
    }
    const target = document.getElementById(id);
    const entry = entries.find(({ panel }) => panel.contains(target));
    return entry && { entry, target };
  }

  function activate(entry, { focus = false, updateHash = false } = {}) {
    for (const { tab, panel } of entries) {
      const selected = panel === entry.panel;
      panel.hidden = !selected;
      tab.setAttribute("aria-selected", String(selected));
      tab.tabIndex = selected ? 0 : -1;
    }
    if (updateHash) window.history.replaceState(null, "", `#${entry.panel.id}`);
    if (focus) entry.tab.focus();
  }

  function revealHash() {
    const match = entryFromHash();
    if (!match) return false;
    activate(match.entry);
    for (
      let parent = match.target.parentElement;
      parent && parent !== match.entry.panel;
      parent = parent.parentElement
    ) {
      if (parent.tagName === "DETAILS") parent.open = true;
    }
    // Fragment navigation may run while its panel is hidden. Scroll only after
    // revealing it, without moving the reader's keyboard focus.
    match.target.scrollIntoView({ block: "start" });
    return true;
  }

  navigation.setAttribute("role", "tablist");
  for (const [index, entry] of entries.entries()) {
    const { tab, panel } = entry;
    tab.setAttribute("role", "tab");
    tab.setAttribute("aria-controls", panel.id);
    panel.setAttribute("role", "tabpanel");
    panel.setAttribute("aria-labelledby", tab.id);
    panel.tabIndex = 0;
    tab.addEventListener("click", (event) => {
      if (event.ctrlKey || event.metaKey || event.shiftKey || event.altKey)
        return;
      event.preventDefault();
      activate(entry, { updateHash: true });
    });
    tab.addEventListener("keydown", (event) => {
      if (event.ctrlKey || event.metaKey || event.altKey || event.shiftKey)
        return;
      let next;
      if (event.key === "ArrowRight") next = (index + 1) % entries.length;
      else if (event.key === "ArrowLeft")
        next = (index - 1 + entries.length) % entries.length;
      else if (event.key === "Home") next = 0;
      else if (event.key === "End") next = entries.length - 1;
      else if (event.key === " ") next = index;
      else return;
      event.preventDefault();
      activate(entries[next], { focus: true, updateHash: true });
    });
  }

  let preferred;
  if (navigation.hasAttribute("data-detect-platform")) {
    const platform = navigator.userAgent;
    preferred = /Win/i.test(platform)
      ? "windows"
      : /Linux/i.test(platform)
        ? "ubuntu"
        : "mac";
  }
  if (!revealHash()) {
    activate(entries.find(({ panel }) => panel.id === preferred) || entries[0]);
  }
  window.addEventListener("hashchange", revealHash);
})();
