import { describe, it, expect } from 'vitest'
import { renderMarkdown, escapeHtml } from '../utils/markdown'

describe('escapeHtml', () => {
  it('escapes angle brackets, ampersands and quotes', () => {
    expect(escapeHtml('<b>&"x"\'</b>')).toBe('&lt;b&gt;&amp;&quot;x&quot;&#39;&lt;/b&gt;')
  })
})

describe('renderMarkdown', () => {
  it('renders bold, italic and inline code', () => {
    const html = renderMarkdown('Use `npm run dev` for **fast** *feedback*')
    expect(html).toContain('<code>npm run dev</code>')
    expect(html).toContain('<strong>fast</strong>')
    expect(html).toContain('<em>feedback</em>')
  })

  it('renders fenced code blocks with escaping', () => {
    const html = renderMarkdown('Intro\n```js\nif (a < b && c > d) {}\n```\nOutro')
    expect(html).toContain('<pre class="md-code">')
    expect(html).toContain('if (a &lt; b &amp;&amp; c &gt; d) {}')
    expect(html).toContain('Intro')
    expect(html).toContain('Outro')
  })

  it('renders headings, lists and blockquotes', () => {
    const html = renderMarkdown('# Title\n- one\n- two\n1. first\n> quoted')
    expect(html).toContain('<h2>Title</h2>')
    expect(html).toContain('<ul><li>one</li><li>two</li></ul>')
    expect(html).toContain('<ol><li>first</li></ol>')
    expect(html).toContain('<blockquote>quoted</blockquote>')
  })

  it('links only http(s) URLs and keeps them inert otherwise', () => {
    const ok = renderMarkdown('[docs](https://example.com/a)')
    expect(ok).toContain('<a href="https://example.com/a" target="_blank" rel="noreferrer">docs</a>')

    const bad = renderMarkdown('[x](javascript:alert(1))')
    expect(bad).not.toContain('<a href="javascript:')

    const img = renderMarkdown('![alt](javascript:alert(1))')
    expect(img).not.toContain('<a href=')
  })

  it('neutralises raw HTML in the source', () => {
    const html = renderMarkdown('<script>alert(1)</script> and <img src=x onerror=y>')
    expect(html).not.toContain('<script>')
    expect(html).not.toContain('<img')
    expect(html).toContain('&lt;script&gt;')
  })

  it('handles empty input', () => {
    expect(renderMarkdown('')).toBe('')
    expect(renderMarkdown(null)).toBe('')
  })
})
