// Run before the first paint. Storage may be unavailable in a private browser.
(() => {
  let saved;
  try { saved = localStorage.getItem('slide-math-site-theme'); } catch {}
  document.documentElement.dataset.theme = ['light', 'dark'].includes(saved)
    ? saved : (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
})();
