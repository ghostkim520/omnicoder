const ESCAPES = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };
const NUL = String.fromCharCode(0);
const FENCE_TOKEN = `${NUL}FENCE`;

export function escapeHtml(value) {
  return String(value ?? '').replace(/[&<>"']/g, (ch) => ESCAPES[ch]);
}

function inline(text) {
  let out = text;
  out = out.replace(/`([^`]+)`/g, '<code>$1</code>');
  out = out.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  out = out.replace(/(^|[^*])\*([^*\n]+)\*/g, '$1<em>$2</em>');
  out = out.replace(
    /\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g,
    '<a href="$2" target="_blank" rel="noreferrer">$1</a>'
  );
  return out;
}

export function renderMarkdown(source) {
  if (!source) return '';
  const fences = [];
  const text = String(source).replace(
    /```[a-zA-Z0-9_+-]*\n?([\s\S]*?)```/g,
    (_match, code) => {
      fences.push(code);
      return `\n${FENCE_TOKEN}${fences.length - 1}${NUL}\n`;
    }
  );
  const lines = escapeHtml(text).split('\n');
  let html = '';
  let paragraph = [];
  let inUl = false;
  let inOl = false;

  const flushParagraph = () => {
    if (paragraph.length) {
      html += `<p>${paragraph.join('<br>')}</p>`;
      paragraph = [];
    }
  };
  const closeLists = () => {
    if (inUl) { html += '</ul>'; inUl = false; }
    if (inOl) { html += '</ol>'; inOl = false; }
  };

  for (const rawLine of lines) {
    const trimmed = rawLine.trim();
    const fence = trimmed.match(new RegExp(`^${FENCE_TOKEN}(\\d+)${NUL}$`));
    if (fence) {
      flushParagraph();
      closeLists();
      html += `<pre class="md-code"><code>${escapeHtml(fences[Number(fence[1])] ?? '')}</code></pre>`;
      continue;
    }
    if (trimmed === '') {
      flushParagraph();
      closeLists();
      continue;
    }
    const heading = trimmed.match(/^(#{1,4})\s+(.*)$/);
    if (heading) {
      flushParagraph();
      closeLists();
      const level = Math.min(heading[1].length + 1, 6);
      html += `<h${level}>${inline(heading[2])}</h${level}>`;
      continue;
    }
    if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
      flushParagraph();
      closeLists();
      html += '<hr>';
      continue;
    }
    const ulItem = trimmed.match(/^[-*]\s+(.*)$/);
    if (ulItem) {
      flushParagraph();
      if (inOl) { html += '</ol>'; inOl = false; }
      if (!inUl) { html += '<ul>'; inUl = true; }
      html += `<li>${inline(ulItem[1])}</li>`;
      continue;
    }
    const olItem = trimmed.match(/^\d+[.)]\s+(.*)$/);
    if (olItem) {
      flushParagraph();
      if (inUl) { html += '</ul>'; inUl = false; }
      if (!inOl) { html += '<ol>'; inOl = true; }
      html += `<li>${inline(olItem[1])}</li>`;
      continue;
    }
    const quote = trimmed.match(/^&gt;\s?(.*)$/);
    if (quote) {
      flushParagraph();
      closeLists();
      html += `<blockquote>${inline(quote[1])}</blockquote>`;
      continue;
    }
    closeLists();
    paragraph.push(inline(trimmed));
  }
  flushParagraph();
  closeLists();

  return html.replace(new RegExp(`${FENCE_TOKEN}(\\d+)${NUL}`, 'g'), (_m, i) => {
    return `<pre class="md-code"><code>${escapeHtml(fences[Number(i)] ?? '')}</code></pre>`;
  });
}
