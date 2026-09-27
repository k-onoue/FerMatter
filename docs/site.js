(() => {
  const root = document.documentElement;
  const toggle = document.getElementById('theme-toggle');
  const screenshot = document.getElementById('app-screenshot');
  const applyTheme = theme => {
    root.dataset.theme = theme;
    toggle.setAttribute('aria-label', `Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`);
    toggle.setAttribute('aria-pressed', String(theme === 'dark'));
    screenshot.src = `assets/app-${theme}.jpg`;
    document.querySelector('meta[name="theme-color"]').content = theme === 'dark' ? '#11141d' : '#f6f7fb';
  };
  applyTheme(root.dataset.theme || 'light');
  toggle.hidden = false;
  toggle.addEventListener('click', () => {
    const theme = root.dataset.theme === 'dark' ? 'light' : 'dark';
    applyTheme(theme);
    try { localStorage.setItem('slide-math-site-theme', theme); } catch {}
  });
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', event => {
    let saved;
    try { saved = localStorage.getItem('slide-math-site-theme'); } catch {}
    if (!['light', 'dark'].includes(saved)) applyTheme(event.matches ? 'dark' : 'light');
  });

  const config = window.MATHLIDELUX_SITE || {};
  // Accept an owner/repository pair, never an arbitrary URL or executable scheme.
  if (typeof config.repository !== 'string' || !/^[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?\/[A-Za-z0-9_.-]+$/.test(config.repository)) return;
  const repository = `https://github.com/${config.repository}`;
  document.querySelectorAll('.repository-link').forEach(link => {
    link.href = repository;
    link.hidden = false;
  });
  document.querySelectorAll('.setup-link').forEach(link => {
    link.href = `${repository}/blob/HEAD/macos-app/README.md`;
    link.hidden = false;
  });
  if (config.releaseAvailable !== true || !/^[A-Za-z0-9][A-Za-z0-9._-]*\.dmg$/.test(config.assetName || '')) return;
  const download = document.getElementById('download-link');
  download.href = `${repository}/releases/latest/download/${encodeURIComponent(config.assetName)}`;
  download.hidden = false;
  document.getElementById('download-pending').hidden = true;
  document.getElementById('release-status').textContent = `Mathlidelux ${config.version} for Mac.`;
})();
