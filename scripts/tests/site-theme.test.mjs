import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';

const script = readFileSync(new URL('../../site/src/scripts/theme.js', import.meta.url), 'utf8');

function page({ light = true, stored = null, blocked = false } = {}) {
  const buttons = [0, 1].map(() => ({
    hidden: true,
    attributes: {},
    setAttribute(name, value) { this.attributes[name] = value; },
    addEventListener(name, handler) { this[name] = handler; },
  }));
  const system = { matches: light, addEventListener(_, handler) { this.change = handler; } };
  const windowEvents = {};
  const documentEvents = {};
  const dataset = {};
  let ready = false;
  runInNewContext(script, {
    matchMedia: () => system,
    localStorage: {
      getItem(key) { assert.equal(key, 'starlight-theme'); if (blocked) throw Error(); return stored; },
      setItem(key, value) { assert.equal(key, 'starlight-theme'); if (blocked) throw Error(); stored = value; },
    },
    window: { addEventListener: (name, handler) => { windowEvents[name] = handler; } },
    document: {
      documentElement: { dataset },
      querySelectorAll: () => ready ? buttons : [],
      addEventListener: (name, handler) => { documentEvents[name] = handler; },
    },
  });
  return {
    dataset, buttons,
    stored: () => stored,
    ready() { ready = true; documentEvents.DOMContentLoaded(); },
    systemChange(light) { system.matches = light; system.change(); },
    storage(value, key = 'starlight-theme') { windowEvents.storage({ key, newValue: value }); },
  };
}

test('initial paint follows the OS for new, legacy auto and invalid preferences', () => {
  for (const stored of [null, '', 'auto', 'invalid']) {
    for (const light of [true, false]) {
      const p = page({ stored, light });
      assert.equal(p.dataset.theme, light ? 'light' : 'dark');
      p.systemChange(!light);
      assert.equal(p.dataset.theme, light ? 'dark' : 'light');
      assert.equal(p.stored(), stored);
    }
  }
});

test('one click toggles, updates all controls, persists and overrides subsequent OS changes', () => {
  const p = page();
  assert.ok(p.buttons.every(button => button.hidden));
  p.ready();
  for (const expected of ['dark', 'light']) {
    p.buttons[0].click();
    assert.equal(p.dataset.theme, expected);
    assert.equal(p.stored(), expected);
    p.systemChange(expected !== 'light');
    assert.equal(p.dataset.theme, expected);
    assert.equal(page({ stored: p.stored(), light: expected !== 'light' }).dataset.theme, expected);
    for (const button of p.buttons) {
      assert.equal(button.hidden, false);
      const label = `Switch to ${expected === 'light' ? 'dark' : 'light'} mode`;
      assert.equal(button.attributes['aria-label'], label);
      assert.equal(button.attributes.title, label);
    }
  }
});

test('blocked storage still permits automatic detection and a stable manual choice', () => {
  const p = page({ blocked: true, light: false });
  assert.equal(p.dataset.theme, 'dark');
  p.ready();
  p.buttons[1].click();
  p.systemChange(false);
  assert.equal(p.dataset.theme, 'light');
});

test('other tabs synchronize preferences; clearing storage returns to OS detection', () => {
  const p = page({ stored: 'dark' });
  p.ready();
  p.storage('light');
  assert.equal(p.dataset.theme, 'light');
  p.storage('dark', 'unrelated');
  assert.equal(p.dataset.theme, 'light');
  p.systemChange(false);
  assert.equal(p.dataset.theme, 'light');
  p.storage(null, null);
  assert.equal(p.dataset.theme, 'dark');
  p.systemChange(true);
  assert.equal(p.dataset.theme, 'light');
});
