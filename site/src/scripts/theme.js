(() => {
  const storageKey = 'starlight-theme';
  const system = matchMedia('(prefers-color-scheme: light)');
  const parse = value => value === 'light' || value === 'dark' ? value : null;
  let preference = null;
  try { preference = parse(localStorage.getItem(storageKey)); } catch {}

  function apply() {
    const theme = preference ?? (system.matches ? 'light' : 'dark');
    document.documentElement.dataset.theme = theme;
    const label = theme === 'light' ? 'Switch to dark mode' : 'Switch to light mode';
    document.querySelectorAll('[data-theme-toggle]').forEach(button => {
      button.setAttribute('aria-label', label);
      button.setAttribute('title', label);
    });
  }

  apply();
  system.addEventListener('change', () => { if (!preference) apply(); });
  window.addEventListener('storage', event => {
    if (event.key === storageKey || event.key === null) {
      preference = parse(event.newValue);
      apply();
    }
  });
  document.addEventListener('DOMContentLoaded', () => {
    document.querySelectorAll('[data-theme-toggle]').forEach(button => {
      button.addEventListener('click', () => {
        preference = document.documentElement.dataset.theme === 'light' ? 'dark' : 'light';
        try { localStorage.setItem(storageKey, preference); } catch {}
        apply();
      });
      button.hidden = false;
    });
    apply();
  }, { once: true });
})();
