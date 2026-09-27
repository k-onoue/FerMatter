/* A local, framework-free UI prototype. Rendering is provided by MathJax. */
const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];
const escapeHTML = value => String(value ?? '').replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[char]));
const ICONS = {
  folder: '<path d="M3 7V5a1 1 0 0 1 1-1h5l2 3h9a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7Z"/>',
  file: '<path d="M14 3H5v18h14V8l-5-5Z"/><path d="M14 3v5h5M8 12h8M8 16h5"/>',
  layers: '<rect x="4" y="3" width="16" height="13" rx="2"/><path d="M7 19h10M9 22h6M8 8h8M8 11h5"/>',
  sparkles: '<path d="m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5L12 3ZM20 2v4M18 4h4"/>',
  search: '<circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/>',
  grid: '<rect x="4" y="4" width="6" height="6" rx="1"/><rect x="14" y="4" width="6" height="6" rx="1"/><rect x="4" y="14" width="6" height="6" rx="1"/><rect x="14" y="14" width="6" height="6" rx="1"/>',
  dense: '<rect x="3" y="4" width="4" height="6" rx=".5"/><rect x="10" y="4" width="4" height="6" rx=".5"/><rect x="17" y="4" width="4" height="6" rx=".5"/><rect x="3" y="14" width="4" height="6" rx=".5"/><rect x="10" y="14" width="4" height="6" rx=".5"/><rect x="17" y="14" width="4" height="6" rx=".5"/>',
  'panel-right': '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  x: '<path d="m6 6 12 12M6 18 18 6"/>',
  message: '<path d="M21 4H3v14h5l4 4v-4h9V4Z"/><path d="M7 8h10M7 12h6"/>',
  sort: '<path d="M8 4v16m-4-4 4 4 4-4M15 5h6M15 10h4M15 15h2"/>',
  'chevron-down': '<path d="m7 10 5 5 5-5"/>',
  'arrow-up-right': '<path d="M6 18 18 6M6 6h12v12"/>',
  presentation: '<rect x="3" y="4" width="18" height="12" rx="1"/><path d="M12 16v5M8 21h8M8 12l3-4 3 3 3-4"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7v.5"/>',
  check: '<path d="m5 12 4 4L19 6"/>',
  edit: '<path d="m15 4 5 5M4 15 15 4a3.5 3.5 0 0 1 5 5L9 20H4v-5Z"/>',
  copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M15 8V4H4v11h4"/>',
  download: '<path d="M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5"/>',
  grip: '<circle cx="8" cy="5" r=".6"/><circle cx="8" cy="12" r=".6"/><circle cx="8" cy="19" r=".6"/><circle cx="16" cy="5" r=".6"/><circle cx="16" cy="12" r=".6"/><circle cx="16" cy="19" r=".6"/>',
  warning: '<path d="m12 3 10 18H2L12 3Z"/><path d="M12 9v5M12 17v.5"/>',
  transparent: '<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M4 12h16M12 4v16"/>',
  trash: '<path d="M3 6h18M8 6V3h8v3M5 6l1 15h12l1-15M10 10v7M14 10v7"/>',
  up: '<path d="M12 20V4m-6 6 6-6 6 6"/>',
  down: '<path d="M12 4v16m-6-6 6 6 6-6"/>',
};
const icon = name => `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true">${ICONS[name] || ICONS.file}</svg>`;
function fillIcons(root = document) { $$('[data-icon]', root).forEach(el => { el.innerHTML = icon(el.dataset.icon); }); }

const STORAGE_KEY = 'slide-math-demo-v2';
let saved = {};
let persistenceAvailable = true;
try { saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}') || {}; }
catch { persistenceAvailable = false; }
const defaultCards = structuredClone(window.SAMPLE_CARDS);
const validCard = card => card && typeof card.id === 'string' && typeof card.name === 'string' && typeof card.tex === 'string' && ['inline', 'block'].includes(card.kind) && ['source', 'custom'].includes(card.origin);
let cards = Array.isArray(saved.cards) && saved.cards.length && saved.cards.every(validCard) ? saved.cards : defaultCards;
const svgCache = new Map();
const renderResults = new Map();
let renderQueue = Promise.resolve();
let initialized = false;
let toastTimer;
let dragId = null;
let editor = null;
let editorTimer;
let editorRevision = 0;
let confirmAction = null;
const state = {
  scope: 'all', format: 'all', query: '',
  view: saved.view === 'compact' ? 'compact' : 'cards',
  sort: saved.sort === 'custom' ? 'custom' : 'source',
  selected: cards.some(c => c.id === saved.selected) ? saved.selected : cards.find(c => c.name === 'Trajectory')?.id || cards[0]?.id,
  inspector: window.innerWidth > 800,
  review: false,
  customOrder: Array.isArray(saved.customOrder) ? saved.customOrder.filter(id => cards.some(c => c.id === id)) : [],
  notes: Array.isArray(saved.notes) ? saved.notes.filter(n => n && typeof n.target === 'string' && typeof n.text === 'string') : [],
};
function sourceSorted() { return [...cards].sort((a, b) => a.sourceOrder - b.sourceOrder || a.id.localeCompare(b.id)); }
function syncCustomOrder() {
  state.customOrder = [...new Set(state.customOrder.filter(id => cards.some(c => c.id === id)))];
  for (const card of sourceSorted()) if (!state.customOrder.includes(card.id)) state.customOrder.push(card.id);
}
syncCustomOrder();
function persist() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ cards, selected: state.selected, view: state.view, sort: state.sort, customOrder: state.customOrder, notes: state.notes }));
    persistenceAvailable = true;
    $('#save-status').textContent = 'このブラウザに保存済み';
  } catch {
    persistenceAvailable = false;
    $('#save-status').textContent = '保存できません — この画面内のみ保持';
  }
}
function toast(message) {
  clearTimeout(toastTimer);
  $('#toast').textContent = message;
  $('#toast').hidden = false;
  toastTimer = setTimeout(() => { $('#toast').hidden = true; }, 3400);
}
function normalizeTeX(source) {
  let tex = source.trim().replace(/(?<!\\)%[^\n]*/g, '').replace(/\\label\{[^}]*\}/g, '').replace(/\\tag\*?\{[^}]*\}/g, '');
  if (tex.startsWith('$$') && tex.endsWith('$$')) tex = tex.slice(2, -2);
  else if (tex.startsWith('$') && tex.endsWith('$')) tex = tex.slice(1, -1);
  else if ((tex.startsWith('\\[') && tex.endsWith('\\]')) || (tex.startsWith('\\(') && tex.endsWith('\\)'))) tex = tex.slice(2, -2);
  tex = tex.replace(/\\(?:begin|end)\{equation\*?\}/g, '');
  tex = tex.replace(/\\(begin|end)\{align\*?\}/g, '\\$1{aligned}').replace(/\\(begin|end)\{(?:gather|multline)\*?\}/g, '\\$1{gathered}');
  return tex.trim();
}
async function mathMarkup(tex, kind) {
  const normalized = normalizeTeX(tex);
  if (!normalized) throw new Error('数式を入力してください。');
  const key = `${kind}:${normalized}`;
  if (svgCache.has(key)) return svgCache.get(key);
  const job = renderQueue.then(async () => {
    await MathJax.startup.promise;
    const container = await MathJax.tex2svgPromise(normalized, { display: kind === 'block' });
    const error = container.querySelector('[data-mjx-error], [data-mml-node="merror"]');
    if (error) throw new Error(error.getAttribute('data-mjx-error') || error.textContent || '数式を読み取れませんでした。');
    // MathJax's noundefined extension prints unknown commands as red mtext.
    const undefinedCommand = [...container.querySelectorAll('[data-mml-node="mtext"]')].find(el => el.getAttribute('fill') === 'red' || el.getAttribute('stroke') === 'red');
    if (undefinedCommand) throw new Error(`未定義のコマンド: ${undefinedCommand.textContent || normalized.match(/\\\w+/)?.[0]}`);
    const markup = container.outerHTML;
    svgCache.set(key, markup);
    return markup;
  });
  renderQueue = job.catch(() => {});
  return job;
}
async function renderCardMath(card) {
  try {
    const markup = await mathMarkup(card.tex, card.kind);
    renderResults.set(card.id, { ok: true, markup });
  } catch (error) {
    renderResults.set(card.id, { ok: false, message: error.message });
  }
}
function orderedCards() {
  if (state.sort === 'source') return sourceSorted();
  const ranks = new Map(state.customOrder.map((id, index) => [id, index]));
  return [...cards].sort((a, b) => ranks.get(a.id) - ranks.get(b.id));
}
function visibleCards() {
  const query = state.query.toLocaleLowerCase();
  return orderedCards().filter(card =>
    (state.scope === 'all' || card.origin === state.scope) &&
    (state.format === 'all' || card.kind === state.format) &&
    (!query || [card.name, card.tex, card.file, card.kind, card.origin].join(' ').toLocaleLowerCase().includes(query))
  );
}
function selectedCard() { return cards.find(card => card.id === state.selected); }
function ensureVisibleSelection(visible) {
  if (!visible.some(card => card.id === state.selected)) state.selected = visible[0]?.id || null;
}
function cardHTML(card, index) {
  const result = renderResults.get(card.id);
  const selected = card.id === state.selected;
  const failed = result && !result.ok;
  const badge = failed ? 'Failed' : card.origin === 'custom' ? 'Custom' : card.kind === 'inline' ? 'Inline' : 'Block';
  const sourceIndex = sourceSorted().findIndex(c => c.id === card.id) + 1;
  const classes = ['equation-card', selected ? 'selected' : '', failed ? 'failed' : ''].join(' ');
  const manyLines = /\\\\/.test(card.tex);
  return `<article class="${classes}" data-card-id="${escapeHTML(card.id)}" data-review="equation-card" tabindex="0" role="listitem" aria-label="${escapeHTML(card.name)} · ${badge}${selected ? ' · 選択中' : ''}" draggable="${state.sort === 'custom' || !!result?.ok}">
    <div class="card-top"><span class="card-number">${selected ? '<span class="selection-dot"></span>' : ''}${String(sourceIndex).padStart(2, '0')}</span><span class="card-type ${failed ? 'failed' : card.origin === 'custom' ? 'custom' : card.kind}">${badge}</span></div>
    ${failed ? `<div class="card-failure">${icon('warning')}<span>Render failed</span><small>${escapeHTML(card.demoFailure ? '未定義マクロのサンプル' : result.message)}</small></div>` : `<div class="card-math ${manyLines ? 'many-lines' : ''}">${result?.markup || '<span class="editor-preview-placeholder">読み込み中…</span>'}</div>`}
    <div class="card-bottom"><span class="card-name">${escapeHTML(card.name || 'Untitled equation')}</span><div class="card-origin">${icon(card.origin === 'source' ? 'file' : 'sparkles')}<span class="origin-text">${card.origin === 'source' ? `${escapeHTML(card.file)} : ${card.line}` : card.parentId ? 'Adapted from source' : 'Your custom equation'}</span><span class="mini-action">${state.sort === 'custom' ? '↕ Move' : '↗ SVG'}</span></div></div>
  </article>`;
}
function renderGallery() {
  const visible = visibleCards();
  ensureVisibleSelection(visible);
  const gallery = $('#equation-gallery');
  gallery.classList.toggle('compact', state.view === 'compact');
  gallery.innerHTML = visible.length ? visible.map(cardHTML).join('') : `<div class="empty-state">${icon('search')}<h3>数式が見つかりません</h3><p>検索語やフィルタを変えてみてください。</p><button class="button secondary" id="reset-filters">絞り込みを解除</button></div>`;
  const failures = visible.filter(card => renderResults.get(card.id)?.ok === false).length;
  $('#result-count').innerHTML = `<strong>${visible.length}</strong> ${visible.length === 1 ? 'equation' : 'equations'} <span class="count-dot">·</span> ${failures ? `${failures} failed` : 'Ready for your slides'}`;
  $('#gallery-end').hidden = !visible.length;
  $('#reorder-hint').hidden = state.sort !== 'custom';
  $('#count-all').textContent = cards.length;
  $('#count-source').textContent = cards.filter(c => c.origin === 'source').length;
  $('#count-custom').textContent = cards.filter(c => c.origin === 'custom').length;
  $('#format-count').textContent = cards.filter(c => state.scope === 'all' || c.origin === state.scope).length;
  $('#library-title').innerHTML = `${{ all: 'Equation library', source: 'From the source', custom: 'Custom equations' }[state.scope]}<span class="heading-dot">.</span>`;
  $('#library-description').textContent = { all: '論文の数式を、次のスライドへ。', source: '元の数式と、その文脈をひとつの場所に。', custom: 'あなたの発表に合わせて、少しだけアレンジ。' }[state.scope];
  $$('[data-scope]').forEach(button => { const active = button.dataset.scope === state.scope; button.classList.toggle('active', active); button.setAttribute('aria-pressed', active); });
  $$('[data-format]').forEach(button => { const active = button.dataset.format === state.format; button.classList.toggle('active', active); button.setAttribute('aria-pressed', active); });
  $$('[data-view]').forEach(button => { const active = button.dataset.view === state.view; button.classList.toggle('active', active); button.setAttribute('aria-pressed', active); });
  $('#sort-order').value = state.sort;
  renderDetail();
}
function highlightedTeX(tex) { return escapeHTML(tex).replace(/(\\[a-zA-Z]+)/g, '<span class="tex-command">$1</span>'); }
function renderDetail() {
  const panel = $('#detail-panel');
  $('#app-shell').classList.toggle('inspector-hidden', !state.inspector);
  $('#inspector-toggle').setAttribute('aria-pressed', state.inspector);
  const card = selectedCard();
  if (!card) { panel.innerHTML = `<div class="panel-heading">Equation details<button class="icon-button" data-action="close-detail" aria-label="詳細を閉じる">${icon('x')}</button></div><div class="detail-empty">${icon('layers')}<h2>数式を選択</h2><p>カードをクリックすると、<br>プレビューと元TeXを表示します。</p></div>`; return; }
  const result = renderResults.get(card.id);
  // A custom card can descend from another custom card; follow the chain to its source.
  let contextCard = card;
  const visited = new Set();
  while (contextCard?.origin !== 'source' && contextCard && !visited.has(contextCard.id)) {
    visited.add(contextCard.id);
    contextCard = cards.find(c => c.id === contextCard.parentId);
  }
  if (contextCard?.origin !== 'source') contextCard = null;
  const position = visibleCards().findIndex(c => c.id === card.id);
  const context = contextCard?.context?.map((line, i) => {
    const number = contextCard.contextStart + i;
    return `<div class="context-line ${number >= contextCard.line && number <= contextCard.endLine ? 'current' : ''}"><span class="line-number">${number}</span><span class="line-source">${escapeHTML(line)}</span></div>`;
  }).join('');
  panel.innerHTML = `<div class="panel-heading"><span>Equation details <small>${String(sourceSorted().findIndex(c => c.id === card.id) + 1).padStart(2, '0')}</small></span><button class="icon-button" data-action="close-detail" aria-label="詳細を閉じる" title="詳細を閉じる">${icon('x')}</button></div>
    <div class="detail-preview" ${result?.ok ? 'draggable="true" title="SVGをドラッグ" data-export-drag="true"' : ''}><div class="preview-math">${result?.ok ? result.markup : `<div class="card-failure">${icon('warning')}Render failed</div>`}</div><div class="preview-transparency">${icon('transparent')}Transparent SVG</div></div>
    <div class="detail-content"><div class="detail-title-row"><h2>${escapeHTML(card.name)}</h2></div><div class="detail-badges"><span class="badge">${card.kind === 'inline' ? 'Inline' : 'Block'}</span><span class="badge ${card.origin === 'custom' ? 'custom' : ''}">${card.origin === 'source' ? 'From source' : 'Custom'}</span><span class="badge ready">${icon(result?.ok ? 'check' : 'warning')}${result?.ok ? 'Ready' : 'Failed'}</span></div>
    <div class="detail-actions"><button class="button secondary" data-action="edit">${icon('edit')}Edit</button><button class="button secondary" data-action="duplicate">${icon('copy')}Duplicate</button></div>
    ${state.sort === 'custom' ? `<div class="reorder-actions"><span>カードの位置</span><button class="icon-button" data-action="move-up" aria-label="前へ移動" ${position <= 0 ? 'disabled' : ''}>${icon('up')}</button><button class="icon-button" data-action="move-down" aria-label="後ろへ移動" ${position === visibleCards().length - 1 ? 'disabled' : ''}>${icon('down')}</button></div>` : ''}</div>
    <section class="detail-section"><div class="section-label">LATEX SOURCE<button class="icon-button" data-action="copy-tex" title="TeXをコピー" aria-label="TeXをコピー">${icon('copy')}</button></div><pre class="source-code">${highlightedTeX(card.tex)}</pre>${result && !result.ok ? `<p class="editor-error">${escapeHTML(result.message)}${card.demoFailure ? '<br>未定義マクロの失敗表示サンプルです。Editから修正できます。' : ''}</p>` : ''}</section>
    <section class="detail-section"><div class="section-label">${card.origin === 'custom' && contextCard ? 'ADAPTED FROM' : 'SOURCE CONTEXT'}</div>${contextCard ? `<div class="source-file">${icon('file')}${escapeHTML(contextCard.file)}</div><div class="source-lines">Lines ${contextCard.line}${contextCard.line !== contextCard.endLine ? `–${contextCard.endLine}` : ''}</div><div class="context-code">${context}</div>` : '<p class="source-attribution">このプロジェクトのカスタム数式です。<br>元ソースには含まれません。</p>'}</section>
    <div class="detail-bottom"><button class="button primary full-width export-button" data-action="export" ${!result?.ok ? 'disabled' : ''}>${icon('download')}Save SVG <span style="opacity:.45">↗</span></button><p class="export-caption">背景透過 · ベクター形式<br>保存したSVGをPowerPointへドラッグ</p>${card.origin === 'custom' ? `<button class="delete-custom" data-action="delete">${icon('trash')}カスタムカードを削除</button>` : ''}</div>`;
}
function selectCard(id, openInspector = true) {
  state.selected = id;
  if (openInspector) state.inspector = true;
  renderGallery();
  persist();
}
function clearFilters() {
  state.scope = 'all'; state.format = 'all'; state.query = ''; $('#search').value = '';
  renderGallery();
}
function focusSelected() { $$('[data-card-id]').find(el => el.dataset.cardId === state.selected)?.focus({ preventScroll: true }); }
function downloadBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a'); anchor.href = url; anchor.download = filename;
  document.body.append(anchor); anchor.click(); anchor.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10000);
}
function svgSource(card) {
  const result = renderResults.get(card.id);
  if (!result?.ok) return null;
  const holder = document.createElement('div'); holder.innerHTML = result.markup;
  const svg = holder.querySelector('svg');
  svg.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
  const [, , w, h] = svg.getAttribute('viewBox').split(/\s+/).map(Number);
  svg.setAttribute('width', `${Math.ceil(w * .02)}px`); svg.setAttribute('height', `${Math.ceil(h * .02)}px`);
  svg.style.cssText = 'color:#20291e';
  svg.removeAttribute('role'); svg.removeAttribute('focusable');
  // Explicit colors and path outlines keep the exported SVG independent of page CSS/fonts.
  $$('[fill="currentColor"]', svg).forEach(el => el.setAttribute('fill', '#20291e'));
  $$('[stroke="currentColor"]', svg).forEach(el => el.setAttribute('stroke', '#20291e'));
  const title = document.createElementNS('http://www.w3.org/2000/svg', 'title'); title.textContent = card.name; svg.prepend(title);
  return svg.outerHTML;
}
function filenameFor(card) { return `${(card.name || 'equation').replace(/[\\/:*?"<>|\x00-\x1f]/g, '-').slice(0, 90)}.svg`; }
function exportCard(card) {
  const source = svgSource(card);
  if (!source) return toast('この数式はまだSVGに書き出せません。');
  downloadBlob(new Blob([source], { type: 'image/svg+xml' }), filenameFor(card));
  toast('SVGのダウンロードを開始しました。');
}
async function copyText(value) {
  try { await navigator.clipboard.writeText(value); toast('TeXをコピーしました。'); }
  catch { toast('コピーできませんでした。LATEX SOURCEから選択してコピーできます。'); }
}
function exportDrag(event, card) {
  const svg = svgSource(card);
  if (!svg) { event.preventDefault(); return; }
  event.dataTransfer.effectAllowed = 'copy';
  const uri = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  event.dataTransfer.setData('DownloadURL', `image/svg+xml:${filenameFor(card)}:${uri}`);
  event.dataTransfer.setData('text/uri-list', uri);
  event.dataTransfer.setData('text/plain', card.tex);
}
function openEditor(mode, card = null) {
  if (!initialized) { toast('数式プレビューの準備中です。'); return; }
  editor = { mode, cardId: card?.id, originalName: '', originalTex: '', originalKind: card?.kind || 'block' };
  const editingExisting = mode === 'edit' && card?.origin === 'custom';
  $('#editor-title').textContent = mode === 'new' ? 'New equation' : editingExisting ? 'Edit equation' : 'Make it your own';
  $('#editor-description').textContent = mode === 'new' ? '発表に合わせた数式を、カスタムカードに。' : editingExisting ? 'カスタム数式を編集します。' : '元の数式から、新しいカスタムカードを作成します。';
  $('#equation-name').value = card ? `${card.name}${editingExisting ? '' : ' · custom'}` : '';
  $('#equation-tex').value = card ? card.tex : '';
  $('#equation-format').value = card?.kind || 'block';
  editor.originalName = $('#equation-name').value;
  editor.originalTex = $('#equation-tex').value;
  $('#editor-error').hidden = true;
  $('#save-equation').disabled = true;
  $('#equation-editor').showModal();
  updateEditorPreview();
  setTimeout(() => (mode === 'new' ? $('#equation-name') : $('#equation-tex')).focus(), 0);
}
async function updateEditorPreview() {
  const revision = ++editorRevision;
  const tex = $('#equation-tex').value;
  $('#save-equation').disabled = true;
  $('#editor-error').hidden = true;
  if (!tex.trim()) {
    $('#editor-preview').innerHTML = '<span class="editor-preview-placeholder">数式を入力すると、ここに表示されます。</span>';
    $('#editor-preview-status').textContent = '入力待ち';
    return;
  }
  $('#editor-preview-status').textContent = 'Rendering…';
  try {
    const markup = await mathMarkup(tex, $('#equation-format').value);
    if (revision !== editorRevision || !editor) return;
    $('#editor-preview').innerHTML = markup;
    $('#editor-preview-status').textContent = '✓ Ready';
    $('#save-equation').disabled = false;
  } catch (error) {
    if (revision !== editorRevision || !editor) return;
    $('#editor-preview').innerHTML = '<span class="editor-preview-placeholder">数式を確認してください。</span>';
    $('#editor-preview-status').textContent = 'Failed';
    $('#editor-error').textContent = error.message;
    $('#editor-error').hidden = false;
  }
}
function confirm(title, message, action, label = '続ける') {
  $('#confirm-title').textContent = title; $('#confirm-message').textContent = message;
  $('#confirm-accept').textContent = label;
  confirmAction = action;
  $('#confirm-dialog').showModal();
}
function closeEditor(force = false) {
  if (!editor) return;
  const dirty = $('#equation-name').value !== editor.originalName || $('#equation-tex').value !== editor.originalTex || $('#equation-format').value !== editor.originalKind;
  if (dirty && !force) {
    confirm('変更を破棄しますか？', 'まだ保存していない数式の変更があります。', () => closeEditor(true), '変更を破棄');
    return;
  }
  editor = null; ++editorRevision; clearTimeout(editorTimer);
  $('#equation-editor').close(); focusSelected();
}
async function saveEquation(event) {
  event?.preventDefault();
  if (!editor) return;
  const tex = $('#equation-tex').value.trim();
  if (!tex) { $('#equation-tex').focus(); return; }
  const formState = { ...editor };
  const kind = $('#equation-format').value;
  const name = $('#equation-name').value.trim() || 'Untitled equation';
  $('#save-equation').disabled = true;
  try {
    const markup = await mathMarkup(tex, kind);
    if (!editor || formState.cardId !== editor.cardId || formState.mode !== editor.mode) return;
    const parent = cards.find(c => c.id === formState.cardId);
    let card;
    if (formState.mode === 'edit' && parent?.origin === 'custom') {
      card = parent; Object.assign(card, { tex, kind, name, demoFailure: false });
    } else {
      const ordered = sourceSorted();
      const next = parent ? ordered[ordered.findIndex(c => c.id === parent.id) + 1] : null;
      const sourceOrder = parent ? (parent.sourceOrder + (next?.sourceOrder ?? parent.sourceOrder + 100)) / 2 : Math.max(...cards.map(c => c.sourceOrder), 0) + 100;
      card = { id: `custom-${crypto.randomUUID()}`, origin: 'custom', name, tex, kind, file: null, line: null, endLine: null, context: [], parentId: parent?.id || null, sourceOrder };
      cards.push(card);
      const anchor = state.customOrder.indexOf(parent?.id);
      state.customOrder.splice(anchor >= 0 ? anchor + 1 : state.customOrder.length, 0, card.id);
    }
    renderResults.set(card.id, { ok: true, markup });
    state.selected = card.id; state.inspector = true;
    // Reveal the saved card even if the editor was opened from a source-only filter.
    if (state.scope === 'source') state.scope = 'all';
    if (state.format !== 'all' && state.format !== kind) state.format = kind;
    state.query = ''; $('#search').value = '';
    closeEditor(true); renderGallery(); persist();
    $$('[data-card-id]').find(el => el.dataset.cardId === card.id)?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    toast(formState.mode === 'edit' && parent?.origin === 'custom' ? 'カスタム数式を保存しました。' : 'カスタムカードを作成しました。');
  } catch (error) {
    $('#editor-error').textContent = error.message; $('#editor-error').hidden = false;
  }
}
function moveCard(id, targetId) {
  if (id === targetId) return;
  const from = state.customOrder.indexOf(id), to = state.customOrder.indexOf(targetId);
  if (from < 0 || to < 0) return;
  state.customOrder.splice(from, 1); state.customOrder.splice(to, 0, id);
  state.selected = id; renderGallery(); persist(); focusSelected();
}
function moveSelected(offset) {
  const visible = visibleCards(); const i = visible.findIndex(c => c.id === state.selected);
  if (visible[i + offset]) moveCard(state.selected, visible[i + offset].id);
}
function deleteCard(card) {
  confirm('カスタムカードを削除しますか？', `「${card.name}」を削除します。`, () => {
    cards = cards.filter(c => c.id !== card.id); renderResults.delete(card.id); syncCustomOrder(); renderGallery(); persist(); toast('カスタムカードを削除しました。');
  }, '削除');
}
function toggleReview(force) {
  state.review = typeof force === 'boolean' ? force : !state.review;
  document.body.classList.toggle('review-mode', state.review);
  $('#review-panel').hidden = !state.review;
  $('#review-toggle').classList.toggle('active', state.review);
  $('#review-toggle').setAttribute('aria-pressed', state.review);
  renderNotes();
}
function renderNotes() {
  $('#review-notes').innerHTML = state.notes.length ? state.notes.map((note, i) => `<div class="review-note"><div class="review-note-header"><span>${escapeHTML(note.target)}</span><button data-remove-note="${i}" aria-label="メモ ${i + 1} を削除">×</button></div><p>${escapeHTML(note.text)}</p></div>`).join('') : '<p class="review-notes-empty">まだメモはありません。</p>';
  $('#export-feedback').disabled = !state.notes.length;
}
function exportFeedback() {
  const groups = new Map();
  state.notes.forEach(note => { if (!groups.has(note.target)) groups.set(note.target, []); groups.get(note.target).push(note.text); });
  const text = `# Mathlidelux — UI Feedback\n\nDate: ${new Date().toLocaleDateString('sv-SE')}\nView: ${state.view} / ${state.sort}\n\n` + [...groups].map(([target, notes]) => `## ${target}\n\n${notes.map(note => '- ' + note.replace(/\n/g, '\n  ')).join('\n')}\n`).join('\n');
  downloadBlob(new Blob([text], { type: 'text/markdown;charset=utf-8' }), 'FEEDBACK.md');
  toast('FEEDBACK.mdのダウンロードを開始しました。');
}

function bindEvents() {
  $$('[data-scope]').forEach(button => button.addEventListener('click', () => { state.scope = button.dataset.scope; renderGallery(); $('#gallery-scroll').scrollTop = 0; }));
  $$('[data-format]').forEach(button => button.addEventListener('click', () => { state.format = button.dataset.format; renderGallery(); $('#gallery-scroll').scrollTop = 0; }));
  $$('[data-view]').forEach(button => button.addEventListener('click', () => { state.view = button.dataset.view; renderGallery(); persist(); }));
  $('#search').addEventListener('input', event => { state.query = event.target.value.trim(); renderGallery(); $('#gallery-scroll').scrollTop = 0; });
  $('#sort-order').addEventListener('change', event => { state.sort = event.target.value; renderGallery(); persist(); });
  $('#inspector-toggle').addEventListener('click', () => { state.inspector = !state.inspector; renderDetail(); });
  $('#new-equation').addEventListener('click', () => openEditor('new'));
  $('#equation-gallery').addEventListener('click', event => {
    if (event.target.closest('#reset-filters')) return clearFilters();
    const element = event.target.closest('[data-card-id]');
    if (!element) return;
    if (event.altKey && state.review) { $('#review-target').value = 'equation-card'; $('#review-note').focus(); return; }
    const now = performance.now();
    const last = $('#equation-gallery').lastCardClick;
    const doubleClick = last && last.id === element.dataset.cardId && now - last.time < 350;
    // Re-rendering replaces the DOM card, so use a click pair as well as dblclick.
    $('#equation-gallery').lastCardClick = { id: element.dataset.cardId, time: now };
    selectCard(element.dataset.cardId);
    if (doubleClick) openEditor('edit', selectedCard());
  });
  $('#equation-gallery').addEventListener('dblclick', event => {
    const el = event.target.closest('[data-card-id]');
    if (el && !editor) openEditor('edit', cards.find(c => c.id === el.dataset.cardId));
  });
  $('#equation-gallery').addEventListener('keydown', event => {
    const el = event.target.closest('[data-card-id]'); if (!el) return;
    if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); selectCard(el.dataset.cardId); focusSelected(); }
    if (event.key === 'ArrowRight' || event.key === 'ArrowLeft' || event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const visible = visibleCards(); const i = visible.findIndex(c => c.id === el.dataset.cardId);
      const columns = getComputedStyle($('#equation-gallery')).gridTemplateColumns.split(' ').length;
      const offset = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: columns, ArrowUp: -columns }[event.key];
      if (visible[i + offset]) { selectCard(visible[i + offset].id, false); focusSelected(); $$('[data-card-id]').find(c => c.dataset.cardId === state.selected)?.scrollIntoView({ block: 'nearest' }); }
    }
  });
  $('#detail-panel').addEventListener('click', event => {
    const action = event.target.closest('[data-action]')?.dataset.action; if (!action) return;
    const card = selectedCard();
    if (action === 'close-detail') { state.inspector = false; renderDetail(); }
    else if (!card) return;
    else if (action === 'edit' || action === 'duplicate') openEditor(action, card);
    else if (action === 'export') exportCard(card);
    else if (action === 'copy-tex') copyText(card.tex);
    else if (action === 'move-up') moveSelected(-1);
    else if (action === 'move-down') moveSelected(1);
    else if (action === 'delete' && card.origin === 'custom') deleteCard(card);
  });
  $('#equation-gallery').addEventListener('dragstart', event => {
    const el = event.target.closest('[data-card-id]'); if (!el) return;
    const card = cards.find(c => c.id === el.dataset.cardId);
    if (state.sort === 'source') return exportDrag(event, card);
    dragId = card.id; event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setData('application/x-slide-math-card', card.id); el.classList.add('dragging');
  });
  $('#equation-gallery').addEventListener('dragover', event => {
    const el = event.target.closest('[data-card-id]');
    if (state.sort !== 'custom' || !dragId || !el) return;
    event.preventDefault(); event.dataTransfer.dropEffect = 'move';
    $$('.drop-target').forEach(target => target.classList.remove('drop-target')); el.classList.add('drop-target');
  });
  $('#equation-gallery').addEventListener('drop', event => {
    const el = event.target.closest('[data-card-id]');
    if (state.sort !== 'custom' || !dragId || !el) return;
    event.preventDefault(); moveCard(dragId, el.dataset.cardId); dragId = null;
  });
  $('#equation-gallery').addEventListener('dragend', () => { dragId = null; $$('.drop-target,.dragging').forEach(el => el.classList.remove('drop-target', 'dragging')); });
  $('#detail-panel').addEventListener('dragstart', event => { if (event.target.closest('[data-export-drag]')) exportDrag(event, selectedCard()); });
  $('#equation-tex').addEventListener('input', () => { ++editorRevision; $('#save-equation').disabled = true; clearTimeout(editorTimer); editorTimer = setTimeout(updateEditorPreview, 220); });
  $('#equation-format').addEventListener('change', () => { clearTimeout(editorTimer); updateEditorPreview(); });
  $('#editor-form').addEventListener('submit', saveEquation);
  $('#close-editor').addEventListener('click', () => closeEditor());
  $('#cancel-editor').addEventListener('click', () => closeEditor());
  $('#equation-editor').addEventListener('cancel', event => { event.preventDefault(); closeEditor(); });
  $('#confirm-cancel').addEventListener('click', () => { $('#confirm-dialog').close(); confirmAction = null; });
  $('#confirm-accept').addEventListener('click', () => { $('#confirm-dialog').close(); const action = confirmAction; confirmAction = null; action?.(); });
  $('#review-toggle').addEventListener('click', () => toggleReview());
  $('#close-review').addEventListener('click', () => toggleReview(false));
  $('#review-target').addEventListener('change', () => {
    $$('.review-highlight').forEach(el => el.classList.remove('review-highlight'));
    const target = $$('[data-review]').find(el => el.dataset.review === $('#review-target').value);
    if (target) { target.classList.add('review-highlight'); setTimeout(() => target.classList.remove('review-highlight'), 1200); }
  });
  $('#add-review').addEventListener('click', () => {
    const text = $('#review-note').value.trim(); if (!text) { $('#review-note').focus(); return; }
    state.notes.push({ target: $('#review-target').value, text }); $('#review-note').value = ''; persist(); renderNotes(); toast('改善メモを保存しました。');
  });
  $('#review-notes').addEventListener('click', event => { const button = event.target.closest('[data-remove-note]'); if (!button) return; state.notes.splice(Number(button.dataset.removeNote), 1); persist(); renderNotes(); });
  $('#export-feedback').addEventListener('click', exportFeedback);
  ['show-help', 'prototype-info'].forEach(id => $(`#${id}`).addEventListener('click', () => $('#info-dialog').showModal()));
  ['close-info', 'dismiss-info'].forEach(id => $(`#${id}`).addEventListener('click', () => $('#info-dialog').close()));
  document.addEventListener('keydown', event => {
    const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(event.target.tagName) || event.target.isContentEditable;
    const modal = $('dialog[open]');
    if (modal) {
      if (modal === $('#equation-editor') && (event.metaKey || event.ctrlKey) && event.key === 'Enter') { event.preventDefault(); if (!$('#save-equation').disabled) saveEquation(); }
      return;
    }
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') { event.preventDefault(); $('#search').focus(); $('#search').select(); return; }
    if (typing) return;
    if (event.key.toLowerCase() === 'r' && !event.metaKey && !event.ctrlKey) toggleReview();
    if (event.key.toLowerCase() === 'n' && !event.metaKey && !event.ctrlKey) openEditor('new');
    if (event.key.toLowerCase() === 'e' && !event.metaKey && !event.ctrlKey && selectedCard()) openEditor('edit', selectedCard());
    if (event.key === 'Escape') { if (state.review) toggleReview(false); else { state.inspector = false; renderDetail(); } }
  });
}
async function init() {
  fillIcons(); bindEvents();
  if (!persistenceAvailable) $('#save-status').textContent = 'この画面内のみ保持';
  try {
    if (!window.MathJax?.startup) throw new Error('MathJaxを読み込めませんでした。npm install を実行して再読み込みしてください。');
    await MathJax.startup.promise;
    await Promise.all(cards.map(renderCardMath));
    initialized = true;
    renderGallery();
    if (persistenceAvailable) persist();
  } catch (error) {
    $('#equation-gallery').innerHTML = `<div class="empty-state">${icon('warning')}<h3>プレビューの準備に失敗しました</h3><p>${escapeHTML(error.message)}</p></div>`;
    $('#result-count').textContent = 'Preview unavailable';
  }
}
init();
