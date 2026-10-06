import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import App from '../App.vue'
import { useChatStore } from '../stores/chatStore'

const commands = []

const TOOLS = [
  { name: 'read_file', risk: 'safe' },
  { name: 'list_dir', risk: 'safe' },
  { name: 'search_files', risk: 'safe' },
  { name: 'write_file', risk: 'risky' },
  { name: 'run_command', risk: 'risky' }
]

let chatHandler = () => ({ content: 'ok', cancelled: false, tool_calls: [] })

function fakeInvoke(cmd, args) {
  commands.push({ cmd, args })
  switch (cmd) {
    case 'gateway_status':
      return Promise.resolve(true)
    case 'list_models':
      return Promise.resolve(['gemini/gemini-3.1-flash-lite'])
    case 'list_tools':
      return Promise.resolve(TOOLS)
    case 'exec_tool':
      return Promise.resolve({ ok: true, output: 'file1.rs\nfile2.rs' })
    case 'chat_cancel':
      return Promise.resolve(null)
    case 'chat':
      return Promise.resolve(chatHandler(args))
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
  return { wrapper, pinia }
}

async function sendMessage(wrapper, text) {
  await wrapper.find('.composer-input').setValue(text)
  await wrapper.find('.btn-send').trigger('click')
  await flushPromises()
  await flushPromises()
}

describe('chat agent loop', () => {
  beforeEach(() => {
    commands.length = 0
    localStorage.clear()
    chatHandler = () => ({ content: 'ok', cancelled: false, tool_calls: [] })
    window.__TAURI_INTERNALS__ = { invoke: fakeInvoke }
    document.body.innerHTML = ''
  })

  it('sends a message and renders the streamed-in reply as markdown', async () => {
    const { wrapper } = await mountApp()

    await sendMessage(wrapper, 'hello there')

    const userBubble = wrapper.find('.msg-user .msg-content')
    expect(userBubble.text()).toBe('hello there')

    const assistant = wrapper.find('.msg-assistant .msg-content')
    expect(assistant.exists()).toBe(true)
    expect(assistant.text()).toContain('ok')

    const call = commands.find(c => c.cmd === 'chat')
    expect(call).toBeTruthy()
    expect(call.args.useTools).toBe(true)
    expect(typeof call.args.runId).toBe('string')
    expect(call.args.model).toBe('gemini/gemini-3.1-flash-lite')
    expect(call.args.messages.at(-1)).toEqual({ role: 'user', content: 'hello there' })
    expect(wrapper.find('.chat-error').exists()).toBe(false)
  })

  it('shows the title from the first message and persists conversations', async () => {
    const { wrapper, pinia } = await mountApp()
    await sendMessage(wrapper, 'List my project files please')

    expect(wrapper.find('.chat-item-title').text()).toContain('List my project files')

    const stored = JSON.parse(localStorage.getItem('omnicoder.chats.v1'))
    expect(stored[0].messages).toHaveLength(2)

    const chat = useChatStore()
    expect(chat.conversations[0].title).toBe('List my project files please')
  })

  it('runs a risky tool only after approval in ask mode', async () => {
    let chatCalls = 0
    chatHandler = () => {
      chatCalls += 1
      if (chatCalls === 1) {
        return {
          content: '',
          cancelled: false,
          tool_calls: [
            {
              id: 'c1',
              type: 'function',
              function: { name: 'run_command', arguments: JSON.stringify({ command: 'ls' }) }
            }
          ]
        }
      }
      return { content: 'All done.', cancelled: false, tool_calls: [] }
    }

    const { wrapper } = await mountApp()
    await sendMessage(wrapper, 'run ls for me')

    const approval = wrapper.find('.approval')
    expect(approval.exists()).toBe(true)
    expect(approval.text()).toContain('run_command')
    expect(commands.filter(c => c.cmd === 'exec_tool')).toHaveLength(0)

    await wrapper.find('.btn-allow').trigger('click')
    await flushPromises()
    await flushPromises()

    const exec = commands.find(c => c.cmd === 'exec_tool')
    expect(exec).toBeTruthy()
    expect(exec.args.name).toBe('run_command')
    expect(JSON.parse(exec.args.args)).toEqual({ command: 'ls' })

    const toolCard = wrapper.find('.tool-card')
    expect(toolCard.text()).toContain('run_command')
    expect(toolCard.text()).toContain('done')

    const replies = wrapper.findAll('.msg-assistant .msg-content')
    expect(replies.at(-1).text()).toContain('All done.')
    expect(wrapper.find('.approval').exists()).toBe(false)
  })

  it('denies risky tools in read-only mode without calling the backend', async () => {
    let chatCalls = 0
    chatHandler = () => {
      chatCalls += 1
      if (chatCalls === 1) {
        return {
          content: '',
          cancelled: false,
          tool_calls: [
            {
              id: 'c1',
              type: 'function',
              function: { name: 'run_command', arguments: JSON.stringify({ command: 'rm -rf /' }) }
            }
          ]
        }
      }
      return { content: 'OK', cancelled: false, tool_calls: [] }
    }

    const { wrapper } = await mountApp()
    await wrapper.find('.access-select').setValue('readonly')
    await sendMessage(wrapper, 'delete everything')

    expect(commands.filter(c => c.cmd === 'exec_tool')).toHaveLength(0)
    expect(wrapper.find('.approval').exists()).toBe(false)

    const toolCard = wrapper.find('.tool-card')
    expect(toolCard.exists()).toBe(true)
    expect(toolCard.text()).toContain('denied')

    const replies = wrapper.findAll('.msg-assistant .msg-content')
    expect(replies.at(-1).text()).toContain('OK')
    expect(chatCalls).toBe(2)
  })

  it('auto-runs safe tools without approval', async () => {
    let chatCalls = 0
    chatHandler = () => {
      chatCalls += 1
      if (chatCalls === 1) {
        return {
          content: '',
          cancelled: false,
          tool_calls: [
            {
              id: 'c1',
              type: 'function',
              function: { name: 'list_dir', arguments: JSON.stringify({ path: '.' }) }
            }
          ]
        }
      }
      return { content: 'There are files.', cancelled: false, tool_calls: [] }
    }

    const { wrapper } = await mountApp()
    await sendMessage(wrapper, 'what files are here')

    expect(wrapper.find('.approval').exists()).toBe(false)
    const exec = commands.find(c => c.cmd === 'exec_tool')
    expect(exec).toBeTruthy()
    expect(exec.args.name).toBe('list_dir')

    const toolCard = wrapper.find('.tool-card')
    expect(toolCard.text()).toContain('done')

    const replies = wrapper.findAll('.msg-assistant .msg-content')
    expect(replies.at(-1).text()).toContain('There are files.')
  })

  it('sends tool results back in OpenAI chat shape', async () => {
    let chatCalls = 0
    chatHandler = () => {
      chatCalls += 1
      if (chatCalls === 1) {
        return {
          content: '',
          cancelled: false,
          tool_calls: [
            {
              id: 'call_42',
              type: 'function',
              function: { name: 'read_file', arguments: JSON.stringify({ path: 'a.txt' }) }
            }
          ]
        }
      }
      return { content: 'done reading', cancelled: false, tool_calls: [] }
    }

    const { wrapper } = await mountApp()
    await sendMessage(wrapper, 'read a.txt')
    await flushPromises()

    const chatCalls2 = commands.filter(c => c.cmd === 'chat')
    expect(chatCalls2).toHaveLength(2)

    const second = chatCalls2[1].args.messages
    const assistantMsg = second.find(m => m.role === 'assistant' && m.tool_calls)
    expect(assistantMsg).toBeTruthy()
    expect(assistantMsg.tool_calls[0].id).toBe('call_42')
    expect(assistantMsg.tool_calls[0].function.name).toBe('read_file')

    const toolMsg = second.find(m => m.role === 'tool')
    expect(toolMsg).toBeTruthy()
    expect(toolMsg.tool_call_id).toBe('call_42')
    expect(toolMsg.content).toContain('file1.rs')
  })

  it('stops a run via the Stop button and calls chat_cancel', async () => {
    chatHandler = () => new Promise(resolve =>
      setTimeout(() => resolve({ content: 'too late', cancelled: false, tool_calls: [] }), 50)
    )

    const { wrapper } = await mountApp()
    await wrapper.find('.composer-input').setValue('long task')
    await wrapper.find('.btn-send').trigger('click')
    await flushPromises()

    expect(wrapper.find('.btn-stop').exists()).toBe(true)
    await wrapper.find('.btn-stop').trigger('click')
    await flushPromises()

    const cancel = commands.find(c => c.cmd === 'chat_cancel')
    expect(cancel).toBeTruthy()
    expect(typeof cancel.args.runId).toBe('string')
  })
})
