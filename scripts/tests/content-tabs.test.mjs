import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { runInNewContext } from "node:vm";

const script = readFileSync(
  new URL("../../site/src/scripts/content-tabs.js", import.meta.url),
  "utf8",
);

function page({
  platform = "MacIntel",
  hash = "",
  linux = true,
  fixes = false,
} = {}) {
  const location = { hash };
  const windowEvents = {};
  const nodes = new Map();
  const ids = fixes
    ? ["released", "pending", "earlier"]
    : linux
      ? ["mac", "windows", "ubuntu"]
      : ["mac", "windows"];
  let focused;
  let scrolled;
  const element = (id, attributes = {}) => ({
    id,
    attributes,
    hidden: false,
    events: {},
    setAttribute(name, value) {
      this.attributes[name] = value;
    },
    getAttribute(name) {
      return this.attributes[name] ?? null;
    },
    hasAttribute(name) {
      return name in this.attributes;
    },
    addEventListener(name, handler) {
      this.events[name] = handler;
    },
    focus() {
      focused = id;
    },
    scrollIntoView() {
      scrolled = id;
    },
  });
  const entries = ids.map((id) => {
    const panel = element(id);
    panel.contains = (node) => node === panel || node?.owner === id;
    const tab = element(`download-tab-${id}`, { href: `#${id}` });
    nodes.set(id, panel);
    nodes.set(tab.id, tab);
    return { tab, panel };
  });
  for (const [id, owner] of [
    ["install", "mac"],
    ["mac-install", "mac"],
    ["typst", "mac"],
    ["typst-on-windows", "windows"],
    ["windows-update", "windows"],
    ["windows-install", "windows"],
    ["ubuntu-install", "ubuntu"],
    ["ubuntu-update", "ubuntu"],
    ["fixes-available-in-downloads", "released"],
    ["fixed-in-source-awaiting-a-download", "pending"],
    ["already-included-in-the-first-public-macos-preview", "earlier"],
  ]) {
    nodes.set(id, { ...element(id), owner });
  }
  const navigation = element(
    "navigation",
    fixes ? {} : { "data-detect-platform": "" },
  );
  navigation.querySelectorAll = () => entries.map(({ tab }) => tab);
  runInNewContext(script, {
    navigator: { userAgent: platform },
    document: {
      querySelector: () => navigation,
      getElementById: (id) => nodes.get(id) ?? null,
    },
    window: {
      location,
      history: {
        replaceState(_state, _unused, hash) {
          location.hash = hash;
        },
      },
      addEventListener: (name, handler) => {
        windowEvents[name] = handler;
      },
    },
  });
  return {
    entries,
    navigation,
    location,
    focused: () => focused,
    scrolled: () => scrolled,
    nodes,
    selected: () => entries.find(({ panel }) => !panel.hidden)?.panel.id,
    hashChange(hash) {
      location.hash = hash;
      windowEvents.hashchange();
    },
    key(id, key, modifiers = {}) {
      let prevented = false;
      nodes.get(`download-tab-${id}`).events.keydown({
        key,
        ...modifiers,
        preventDefault: () => (prevented = true),
      });
      return prevented;
    },
    click(id, modifiers = {}) {
      let prevented = false;
      nodes.get(`download-tab-${id}`).events.click({
        ...modifiers,
        preventDefault: () => (prevented = true),
      });
      return prevented;
    },
  };
}

test("OS detection chooses a default without changing the address or focus", () => {
  for (const [platform, expected] of [
    ["MacIntel", "mac"],
    ["Win32", "windows"],
    ["Linux x86_64", "ubuntu"],
  ]) {
    const p = page({ platform });
    assert.equal(p.selected(), expected);
    assert.equal(p.location.hash, "");
    assert.equal(p.focused(), undefined);
    assert.equal(p.scrolled(), undefined);
    assert.equal(p.navigation.attributes.role, "tablist");
    for (const { tab, panel } of p.entries) {
      assert.equal(tab.attributes.role, "tab");
      assert.equal(tab.attributes["aria-controls"], panel.id);
      assert.equal(panel.attributes.role, "tabpanel");
      assert.equal(panel.attributes["aria-labelledby"], tab.id);
      assert.equal(tab.tabIndex, panel.id === expected ? 0 : -1);
      assert.equal(
        tab.attributes["aria-selected"],
        String(panel.id === expected),
      );
    }
  }
});

test("existing section and nested hashes take precedence over OS detection", () => {
  for (const [hash, expected] of [
    ["#mac", "mac"],
    ["#windows", "windows"],
    ["#ubuntu", "ubuntu"],
    ["#install", "mac"],
    ["#mac-install", "mac"],
    ["#typst", "mac"],
    ["#typst-on-windows", "windows"],
    ["#windows-update", "windows"],
    ["#windows-install", "windows"],
    ["#ubuntu-install", "ubuntu"],
    ["#ubuntu-update", "ubuntu"],
  ]) {
    const p = page({ platform: "Win32", hash });
    assert.equal(p.selected(), expected);
    assert.equal(p.location.hash, hash);
    assert.equal(p.scrolled(), hash.slice(1));
  }
});

test("hash navigation reveals the requested content without taking keyboard focus", () => {
  const p = page();
  p.hashChange("#windows-update");
  assert.equal(p.selected(), "windows");
  assert.equal(p.scrolled(), "windows-update");
  p.hashChange("#ubuntu-update");
  assert.equal(p.selected(), "ubuntu");
  p.hashChange("#unrelated");
  assert.equal(p.selected(), "ubuntu");
  assert.equal(p.focused(), undefined);
});

test("deep links open collapsed instructions before scrolling to the target", () => {
  const p = page();
  const details = { tagName: "DETAILS", open: false };
  p.nodes.get("typst-on-windows").parentElement = details;
  p.hashChange("#typst-on-windows");
  assert.equal(p.selected(), "windows");
  assert.equal(details.open, true);
  assert.equal(p.scrolled(), "typst-on-windows");
  assert.equal(p.focused(), undefined);
});

test("arrow keys wrap, Home/End select edges, and Space activates a focused tab", () => {
  const p = page();
  for (const [from, key, expected] of [
    ["mac", "ArrowLeft", "ubuntu"],
    ["ubuntu", "ArrowRight", "mac"],
    ["mac", "End", "ubuntu"],
    ["ubuntu", "Home", "mac"],
    ["mac", "ArrowRight", "windows"],
    ["windows", " ", "windows"],
  ]) {
    assert.equal(p.key(from, key), true);
    assert.equal(p.selected(), expected);
    assert.equal(p.focused(), `download-tab-${expected}`);
    assert.equal(p.location.hash, `#${expected}`);
  }
  assert.equal(p.key("windows", "Tab"), false);
});

test("modified arrow keys retain browser navigation behavior", () => {
  const p = page();
  for (const modifier of ["ctrlKey", "metaKey", "altKey", "shiftKey"]) {
    assert.equal(p.key("mac", "ArrowLeft", { [modifier]: true }), false);
    assert.equal(p.selected(), "mac");
    assert.equal(p.focused(), undefined);
    assert.equal(p.location.hash, "");
  }
});

test("click selects a panel while modified clicks retain normal link behavior", () => {
  const p = page();
  assert.equal(p.click("windows"), true);
  assert.equal(p.selected(), "windows");
  assert.equal(p.location.hash, "#windows");
  for (const modifier of ["ctrlKey", "metaKey", "shiftKey", "altKey"]) {
    assert.equal(p.click("mac", { [modifier]: true }), false);
    assert.equal(p.selected(), "windows");
  }
});

test("unavailable Linux and malformed fragments retain a readable default", () => {
  const p = page({ platform: "Linux x86_64", hash: "#%broken", linux: false });
  assert.equal(p.selected(), "mac");
  assert.equal(p.key("mac", "ArrowRight"), true);
  assert.equal(p.selected(), "windows");
  assert.equal(p.key("windows", "ArrowRight"), true);
  assert.equal(p.selected(), "mac");
});

test("fix categories show exactly one group, independent of the reader's OS", () => {
  for (const platform of ["MacIntel", "Win32", "Linux x86_64"]) {
    const p = page({ fixes: true, platform });
    assert.equal(p.selected(), "released");
    for (const id of ["pending", "earlier", "released"]) {
      p.click(id);
      assert.deepEqual(
        p.entries
          .filter(({ panel }) => !panel.hidden)
          .map(({ panel }) => panel.id),
        [id],
      );
      assert.equal(p.location.hash, `#${id}`);
    }
    p.key("released", "ArrowLeft");
    assert.equal(p.selected(), "earlier");
    p.key("earlier", "Home");
    assert.equal(p.selected(), "released");
  }
});

test("fix category and legacy fragments reveal their group on load and hash navigation", () => {
  for (const [hash, expected] of [
    ["#pending", "pending"],
    ["#earlier", "earlier"],
    ["#fixes-available-in-downloads", "released"],
    ["#fixed-in-source-awaiting-a-download", "pending"],
    ["#already-included-in-the-first-public-macos-preview", "earlier"],
  ]) {
    const p = page({ fixes: true, hash });
    assert.equal(p.selected(), expected);
    assert.equal(p.scrolled(), hash.slice(1));
    assert.equal(p.focused(), undefined);
    p.hashChange("#released");
    assert.equal(p.selected(), "released");
    p.hashChange(hash);
    assert.equal(p.selected(), expected);
  }
});
