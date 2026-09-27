(() => {
  const root = document.documentElement;
  const toggle = document.getElementById('theme-toggle');
  const applyTheme = theme => {
    root.dataset.theme = theme;
    if (toggle) {
      toggle.textContent = theme === 'dark' ? 'Light mode' : 'Dark mode';
      toggle.setAttribute('aria-label', `Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`);
      toggle.setAttribute('aria-pressed', String(theme === 'dark'));
    }
    const meta = document.querySelector('meta[name="theme-color"]');
    if (meta) meta.content = theme === 'dark' ? '#141b24' : '#f7f8fa';
  };
  applyTheme(root.dataset.theme || (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'));
  if (toggle) {
    toggle.hidden = false;
    toggle.addEventListener('click', () => {
      const theme = root.dataset.theme === 'dark' ? 'light' : 'dark';
      applyTheme(theme);
      try { localStorage.setItem('slide-math-site-theme', theme); } catch {}
    });
  }
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', event => {
    let saved;
    try { saved = localStorage.getItem('slide-math-site-theme'); } catch {}
    if (!['light', 'dark'].includes(saved)) applyTheme(event.matches ? 'dark' : 'light');
  });

  const config = window.MATHLIDELUX_SITE || {};
  const download = document.getElementById('download-link');
  if (!download || config.releaseAvailable !== true) return;
  // A configured repository pair and DMG filename cannot introduce arbitrary URLs.
  if (typeof config.repository !== 'string' || !/^[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?\/[A-Za-z0-9_.-]+$/.test(config.repository)) return;
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]*\.dmg$/.test(config.assetName || '')) return;
  download.href = `https://github.com/${config.repository}/releases/latest/download/${encodeURIComponent(config.assetName)}`;
  download.hidden = false;
  document.getElementById('download-pending').hidden = true;
  document.getElementById('release-status').textContent = `Mathlidelux ${config.version} for Mac.`;
})();
