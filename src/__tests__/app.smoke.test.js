import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia } from 'pinia'
import App from '../App.vue'

const commands = []

function fakeInvoke(cmd, args) {
  commands.push({ cmd, args })
  switch (cmd) {
    case 'gateway_status':
      return Promise.resolve(true)
    case 'list_models':
      return Promise.resolve([
        'mistral/codestral-2508',
        'mistral/codestral-latest',
        'mistral/mistral-large-latest',
        'gemini/gemini-3.1-flash-lite'
      ])
    case 'generate_code':
      return Promise.resolve({ code: 'def add(a, b):\n    return a + b\n', error: null })
    case 'debug_code':
      return Promise.resolve('No issues found.')
    case 'run_command':
      return Promise.resolve('$ ls\nmain.rs')
    case 'git_status':
      return Promise.resolve('On branch main')
    case 'git_commit':
    case 'git_push':
      return Promise.resolve(null)
    case 'list_tools':
      return Promise.resolve([
        { name: 'read_file', risk: 'safe' },
        { name: 'list_dir', risk: 'safe' },
        { name: 'search_files', risk: 'safe' },
        { name: 'write_file', risk: 'risky' },
        { name: 'run_command', risk: 'risky' }
      ])
    case 'exec_tool':
      return Promise.resolve({ ok: true, output: 'tool ok' })
    case 'chat_cancel':
      return Promise.resolve(null)
    case 'chat':
      return Promise.resolve({ content: 'Hi!', cancelled: false, tool_calls: [] })
    default:
      return Promise.reject(new Error('unknown command: ' + cmd))
  }
}

async function mountApp() {
  const pinia = createPinia()
  const wrapper = mount(App, {
    global: { plugins: [pinia] },
    attachTo: document.body
  })
  await flushPromises()
  await flushPromises()
  return wrapper
}

describe('OmniCoder app', () => {
  beforeEach(() => {
    commands.length = 0
    window.__TAURI_INTERNALS__ = { invoke: fakeInvoke }
    document.body.innerHTML = ''
  })

  it('renders header, providers and real model list', async () => {
    const wrapper = await mountApp()

    expect(wrapper.find('header h1').text()).toBe('OmniCoder')

    const providerNames = wrapper.findAll('.provider-card h3').map(h => h.text())
    expect(providerNames).toContain('OmniRoute Gateway')

    const status = wrapper.find('.status-online').text()
    expect(status).toBe('Online')

    const options = wrapper.findAll('.model-select option').map(o => o.attributes('value'))
    expect(options).toContain('mistral/codestral-latest')
    expect(options).toContain('gemini/gemini-3.1-flash-lite')

    expect(commands.map(c => c.cmd)).toEqual(
      expect.arrayContaining(['gateway_status', 'list_models'])
    )
  })

  it('generates code through the backend and shows it', async () => {
    const wrapper = await mountApp()

    const textarea = wrapper.find('.code-editor textarea')
    await textarea.setValue('Write an add function')

    const button = wrapper.findAll('button').find(b => b.text() === 'Generate Code')
    await button.trigger('click')
    await flushPromises()

    const generated = wrapper.find('.generated-code pre')
    expect(generated.exists()).toBe(true)
    expect(generated.text()).toContain('def add')

    const call = commands.find(c => c.cmd === 'generate_code')
    expect(call).toBeTruthy()
    expect(call.args.prompt).toBe('Write an add function')
    expect(call.args.model).toBe('gemini/gemini-3.1-flash-lite')
    expect(call.args.maxTokens).toBe(4096)
    expect(call.args.useLocal).toBe(false)
    expect(wrapper.find('.error-message').exists()).toBe(false)
  })

  it('surfaces backend errors instead of failing silently', async () => {
    window.__TAURI_INTERNALS__ = {
      invoke: (cmd) => {
        if (cmd === 'generate_code') {
          return Promise.resolve({ code: null, error: '{"error":{"message":"[402]: API budget exhausted."}}' })
        }
        return fakeInvoke(cmd)
      }
    }
    const wrapper = await mountApp()

    await wrapper.find('.code-editor textarea').setValue('x')
    const button = wrapper.findAll('button').find(b => b.text() === 'Generate Code')
    await button.trigger('click')
    await flushPromises()

    expect(wrapper.find('.error-message').text()).toContain('402')
    expect(wrapper.find('.generated-code').exists()).toBe(false)
  })
})
