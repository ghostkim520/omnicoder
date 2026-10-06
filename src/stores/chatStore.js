import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAIStore } from './aiStore';

const CHATS_KEY = 'omnicoder.chats.v1';
const MODE_KEY = 'omnicoder.pcAccess.v1';
const MAX_TURNS = 8;
const HISTORY_LIMIT = 40;

let approvalResolver = null;
let listenerPromise = null;
let listenerBroken = false;
let cancelled = false;

function uuid() {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) return crypto.randomUUID();
  return `id-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function freshConversation() {
  return { id: uuid(), title: 'New chat', createdAt: Date.now(), messages: [] };
}

function loadConversations() {
  try {
    const raw = localStorage.getItem(CHATS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed) && parsed.length && Array.isArray(parsed[0].messages)) {
        return parsed;
      }
    }
  } catch (e) {
    /* corrupted storage → start fresh */
  }
  return [freshConversation()];
}

function safeParseArgs(raw) {
  if (!raw) return {};
  if (typeof raw === 'object') return raw;
  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === 'object' ? parsed : { raw: parsed };
  } catch (e) {
    return { _raw: String(raw) };
  }
}

export const useChatStore = defineStore('chat', {
  state: () => {
    const conversations = loadConversations();
    return {
      conversations,
      activeId: conversations[0].id,
      pcAccess: localStorage.getItem(MODE_KEY) || 'ask',
      sending: false,
      error: null,
      streamText: '',
      streamReasoning: '',
      pendingApproval: null,
      toolCatalog: [],
      currentRunId: null,
    };
  },
  getters: {
    activeConversation(state) {
      return state.conversations.find((c) => c.id === state.activeId) || state.conversations[0];
    },
    riskOf() {
      return (name) => {
        const def = this.toolCatalog.find((t) => t.name === name);
        return def ? def.risk : 'risky';
      };
    },
  },
  actions: {
    persist() {
      try {
        localStorage.setItem(CHATS_KEY, JSON.stringify(this.conversations.slice(0, 30)));
      } catch (e) {
        /* storage full/unavailable → keep in memory */
      }
    },

    newChat() {
      const conv = freshConversation();
      this.conversations.unshift(conv);
      this.activeId = conv.id;
      this.error = null;
      this.persist();
    },

    selectChat(id) {
      if (this.conversations.some((c) => c.id === id)) {
        this.activeId = id;
        this.error = null;
      }
    },

    setAccess(mode) {
      this.pcAccess = mode;
      try {
        localStorage.setItem(MODE_KEY, mode);
      } catch (e) {
        /* ignore */
      }
    },

    resolveApproval(allow) {
      if (approvalResolver) {
        const resolve = approvalResolver;
        approvalResolver = null;
        resolve(allow ? 'allow' : 'deny');
      }
    },

    async stop() {
      cancelled = true;
      if (approvalResolver) this.resolveApproval(false);
      const runId = this.currentRunId;
      if (runId) {
        try {
          await invoke('chat_cancel', { runId });
        } catch (e) {
          /* ignore */
        }
      }
    },

    async ensureListener() {
      if (listenerBroken || listenerPromise) return listenerPromise;
      listenerPromise = (async () => {
        try {
          await listen('chat-chunk', (event) => {
            const payload = (event && event.payload) || {};
            if (payload.run_id !== this.currentRunId) return;
            if (payload.content) this.streamText += payload.content;
            if (payload.reasoning) this.streamReasoning += payload.reasoning;
          });
        } catch (e) {
          listenerBroken = true;
          listenerPromise = null;
        }
      })();
      return listenerPromise;
    },

    async send(rawText) {
      const text = String(rawText || '').trim();
      if (!text || this.sending) return;
      const conv = this.activeConversation;
      conv.messages.push({ id: uuid(), role: 'user', content: text });
      if (conv.title === 'New chat') {
        conv.title = text.length > 40 ? `${text.slice(0, 40)}…` : text;
      }
      this.persist();
      await this.runLoop(conv);
    },

    buildApiMessages(conv) {
      const out = [];
      const messages = conv.messages.slice(-HISTORY_LIMIT);
      for (const m of messages) {
        if (m.role === 'user') {
          out.push({ role: 'user', content: m.content });
          continue;
        }
        if (m.role !== 'assistant') continue;
        const executed = (m.toolCalls || []).filter((tc) => tc.output);
        const entry = { role: 'assistant', content: m.content || '' };
        if (executed.length) {
          entry.tool_calls = executed.map((tc) => ({
            id: tc.id,
            type: 'function',
            function: { name: tc.name, arguments: JSON.stringify(tc.args || {}) },
          }));
        }
        out.push(entry);
        for (const tc of executed) {
          out.push({
            role: 'tool',
            tool_call_id: tc.id,
            content: `[${tc.status}] ${tc.output}`,
          });
        }
      }
      while (out.length && out[0].role === 'tool') out.shift();
      return out;
    },

    async executeTool(tc) {
      const risk = this.riskOf(tc.name);
      const mode = this.pcAccess;
      if (risk === 'risky') {
        if (mode === 'readonly') {
          tc.status = 'denied';
          tc.output =
            'Denied: PC access is "Read-only". Writes and shell commands are blocked — switch PC access to "Ask before changes" or "Full auto" in the header to allow them.';
          return;
        }
        if (mode === 'ask') {
          this.pendingApproval = {
            id: uuid(),
            toolId: tc.id,
            name: tc.name,
            args: tc.args,
          };
          const decision = await new Promise((resolve) => {
            approvalResolver = resolve;
          });
          this.pendingApproval = null;
          if (cancelled || decision !== 'allow') {
            tc.status = 'denied';
            tc.output = 'Denied by the user.';
            return;
          }
        }
      }
      tc.status = 'running';
      try {
        const res = await invoke('exec_tool', {
          name: tc.name,
          args: JSON.stringify(tc.args || {}),
        });
        tc.status = res && res.ok ? 'done' : 'error';
        tc.output = (res && res.output) || '(no output)';
      } catch (e) {
        tc.status = 'error';
        tc.output = String((e && e.message) || e);
      }
    },

    async runLoop(conv) {
      this.sending = true;
      this.error = null;
      cancelled = false;
      approvalResolver = null;
      this.pendingApproval = null;
      await this.ensureListener();
      if (!this.toolCatalog.length) {
        try {
          this.toolCatalog = await invoke('list_tools');
        } catch (e) {
          this.toolCatalog = [];
        }
      }
      const aiStore = useAIStore();
      try {
        for (let turn = 0; turn < MAX_TURNS && !cancelled; turn++) {
          this.currentRunId = uuid();
          this.streamText = '';
          this.streamReasoning = '';
          const apiMessages = this.buildApiMessages(conv);
          let reply;
          try {
            reply = await invoke('chat', {
              messages: apiMessages,
              model: aiStore.selectedModel,
              useTools: true,
              runId: this.currentRunId,
            });
          } catch (e) {
            this.error = String((e && e.message) || e);
            conv.messages.push({
              id: uuid(),
              role: 'assistant',
              content: `**Gateway error:** ${this.error}`,
              error: true,
            });
            break;
          }
          const streamed = this.streamText;
          this.streamText = '';
          this.streamReasoning = '';
          if (reply.cancelled) {
            const partial = reply.content || streamed;
            if (partial) {
              conv.messages.push({
                id: uuid(),
                role: 'assistant',
                content: partial,
                partial: true,
              });
            }
            break;
          }
          const toolCalls = reply.tool_calls || [];
          if (!toolCalls.length) {
            conv.messages.push({
              id: uuid(),
              role: 'assistant',
              content: reply.content || streamed || '(no response)',
            });
            break;
          }
          const assistantMsg = {
            id: uuid(),
            role: 'assistant',
            content: reply.content || '',
            toolCalls: toolCalls.map((tc) => ({
              id: tc.id || uuid(),
              name: tc.function.name,
              args: safeParseArgs(tc.function.arguments),
              status: 'pending',
              output: '',
              open: false,
            })),
          };
          conv.messages.push(assistantMsg);
          for (const tc of assistantMsg.toolCalls) {
            if (cancelled) {
              tc.status = 'denied';
              tc.output = 'Cancelled.';
              break;
            }
            await this.executeTool(tc);
          }
          this.persist();
          if (cancelled) {
            const partial = reply.content || streamed;
            if (partial && !assistantMsg.content) assistantMsg.content = partial;
            break;
          }
        }
      } finally {
        this.sending = false;
        this.currentRunId = null;
        this.streamText = '';
        this.streamReasoning = '';
        this.pendingApproval = null;
        approvalResolver = null;
        this.persist();
      }
    },
  },
});
