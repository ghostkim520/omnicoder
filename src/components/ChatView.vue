<template>
  <div class="chat-view">
    <div ref="scroller" class="chat-scroll">
      <div v-if="!messages.length && !chat.sending" class="chat-empty">
        <div class="chat-empty-badge">OC</div>
        <h2>What should we work on?</h2>
        <p>
          Ask about your code or your machine. I can read and search files, edit them and run
          shell commands — with your approval, depending on the PC access mode.
        </p>
        <div class="suggestions">
          <button
            v-for="s in suggestions"
            :key="s"
            class="suggestion"
            @click="sendSuggestion(s)"
          >
            {{ s }}
          </button>
        </div>
      </div>

      <template v-else>
        <div
          v-for="m in messages"
          :key="m.id"
          :class="['msg', m.role === 'user' ? 'msg-user' : 'msg-assistant', { 'msg-error': m.error, 'msg-partial': m.partial }]"
        >
          <div class="msg-role">{{ m.role === 'user' ? 'You' : 'OmniCoder' }}</div>
          <div class="msg-body">
            <div v-if="m.toolCalls && m.toolCalls.length" class="tool-list">
              <div
                v-for="tc in m.toolCalls"
                :key="tc.id"
                class="tool-card"
                :class="`tool-${tc.status}`"
              >
                <button type="button" class="tool-head" @click="tc.open = !tc.open">
                  <span class="tool-name">{{ tc.name }}</span>
                  <span class="tool-args">{{ argsSummary(tc) }}</span>
                  <span class="tool-status">{{ statusLabel(tc.status) }}</span>
                </button>
                <pre v-if="tc.open && tc.output" class="tool-output">{{ tc.output }}</pre>
              </div>
            </div>
            <div v-if="m.content" class="msg-content" v-html="render(m.content)"></div>
            <details v-if="m.reasoning" class="reasoning">
              <summary>Reasoning</summary>
              <div class="reasoning-body">{{ m.reasoning }}</div>
            </details>
          </div>
        </div>

        <div v-if="chat.sending" class="msg msg-assistant msg-streaming">
          <div class="msg-role">OmniCoder</div>
          <div class="msg-body">
            <details v-if="chat.streamReasoning" class="reasoning" open>
              <summary>Thinking…</summary>
              <div class="reasoning-body">{{ chat.streamReasoning }}</div>
            </details>
            <div v-if="chat.streamText" class="msg-content" v-html="render(chat.streamText)"></div>
            <div v-else-if="!chat.streamReasoning" class="typing" aria-label="thinking">
              <span class="dot" /><span class="dot" /><span class="dot" />
            </div>
          </div>
        </div>
      </template>

      <div v-if="chat.pendingApproval" class="approval">
        <div class="approval-title">
          Allow <code>{{ chat.pendingApproval.name }}</code> to run?
        </div>
        <code class="approval-args">{{ argsSummary(chat.pendingApproval) }}</code>
        <p class="approval-hint">
          This tool can change files or run programs on this computer.
        </p>
        <div class="approval-actions">
          <button class="btn-allow" @click="chat.resolveApproval(true)">Allow</button>
          <button class="btn-deny" @click="chat.resolveApproval(false)">Deny</button>
        </div>
      </div>

      <div v-if="chat.error" class="chat-error">{{ chat.error }}</div>
    </div>

    <div class="composer">
      <div class="composer-meta">
        <span class="access-hint">{{ accessHint }}</span>
        <span v-if="chat.sending" class="working-hint">working…</span>
      </div>
      <div class="composer-row">
        <textarea
          v-model="draft"
          class="composer-input"
          rows="3"
          placeholder="Message OmniCoder… (Enter to send, Shift+Enter for a new line)"
          @keydown.enter.exact.prevent="sendDraft"
        ></textarea>
        <div class="composer-buttons">
          <button
            v-if="chat.sending"
            class="btn-stop"
            title="Stop the current run"
            @click="chat.stop()"
          >
            Stop
          </button>
          <button v-else class="btn-send" :disabled="!draft.trim()" @click="sendDraft()">
            Send
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import { ref, computed, watch, nextTick } from 'vue';
import { useChatStore } from '../stores/chatStore';
import { renderMarkdown } from '../utils/markdown';

const SUGGESTIONS = [
  'What is in my home directory?',
  'Find my most recently modified files',
  'Run git status in my project',
  'Explain what this app does',
];

export default {
  setup() {
    const chat = useChatStore();
    const draft = ref('');
    const scroller = ref(null);

    const messages = computed(() => chat.activeConversation?.messages || []);

    const accessHint = computed(() => {
      if (chat.pcAccess === 'readonly') {
        return 'Read-only: files and searches work, writes and commands are blocked.';
      }
      if (chat.pcAccess === 'auto') {
        return 'Full auto: tools run without asking.';
      }
      return 'Ask before changes: reads run automatically; writes and commands wait for approval.';
    });

    const render = (text) => renderMarkdown(text);

    const scrollToBottom = () => {
      nextTick(() => {
        const el = scroller.value;
        if (el) el.scrollTop = el.scrollHeight;
      });
    };

    const argsSummary = (tc) => {
      const args = (tc && tc.args) || {};
      const parts = Object.entries(args)
        .filter(([k]) => k !== 'content')
        .map(([k, v]) => `${k}=${typeof v === 'string' ? v : JSON.stringify(v)}`);
      const summary = parts.join(' ');
      const content = args.content;
      if (typeof content === 'string' && content.length > 0) {
        return `${summary} content(${content.length} chars)`.trim();
      }
      return summary || '…';
    };

    const statusLabel = (status) =>
      ({ pending: 'waiting', running: 'running…', done: 'done', error: 'failed', denied: 'denied' }[status] ||
        status);

    const sendDraft = () => {
      const text = draft.value.trim();
      if (!text || chat.sending) return;
      draft.value = '';
      chat.send(text);
    };

    const sendSuggestion = (s) => {
      if (chat.sending) return;
      chat.send(s);
    };

    watch(
      () => [
        messages.value.length,
        chat.sending,
        chat.streamText.length,
        chat.streamReasoning.length,
        chat.pendingApproval && chat.pendingApproval.id,
      ],
      scrollToBottom
    );

    return {
      chat,
      draft,
      scroller,
      messages,
      accessHint,
      suggestions: SUGGESTIONS,
      render,
      argsSummary,
      statusLabel,
      sendDraft,
      sendSuggestion,
    };
  },
};
</script>

<style scoped>
.chat-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #161616;
  color: #e6e6e6;
}
.chat-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 24px 20px 12px;
}
.chat-empty {
  max-width: 640px;
  margin: 8vh auto 0;
  text-align: center;
}
.chat-empty-badge {
  display: inline-block;
  width: 52px;
  height: 52px;
  line-height: 52px;
  border-radius: 14px;
  background: #007acc;
  color: #fff;
  font-weight: 700;
  margin-bottom: 12px;
}
.chat-empty h2 {
  margin: 0 0 8px;
  font-size: 22px;
}
.chat-empty p {
  margin: 0 0 20px;
  color: #a9a9a9;
  line-height: 1.5;
}
.suggestions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
}
.suggestion {
  padding: 8px 14px;
  background: #222;
  border: 1px solid #3a3a3a;
  border-radius: 999px;
  color: #d0d0d0;
  cursor: pointer;
  font-size: 13px;
}
.suggestion:hover {
  border-color: #007acc;
}
.msg {
  max-width: 860px;
  margin: 0 auto 18px;
}
.msg-role {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #8a8a8a;
  margin-bottom: 4px;
}
.msg-user .msg-body {
  background: #1d3a52;
  border: 1px solid #2c5f8a;
}
.msg-assistant .msg-body {
  background: #202020;
  border: 1px solid #333;
}
.msg-body {
  border-radius: 10px;
  padding: 12px 14px;
  overflow-wrap: anywhere;
}
.msg-content :deep(p) {
  margin: 0 0 8px;
}
.msg-content :deep(p:last-child) {
  margin-bottom: 0;
}
.msg-content :deep(pre) {
  background: #0d0d0d;
  border: 1px solid #333;
  border-radius: 8px;
  padding: 10px 12px;
  overflow-x: auto;
  font-size: 13px;
}
.msg-content :deep(code) {
  background: #2a2a2a;
  border-radius: 4px;
  padding: 1px 5px;
  font-size: 0.92em;
}
.msg-content :deep(pre code) {
  background: none;
  padding: 0;
}
.msg-error .msg-body {
  border-color: #a13d3d;
}
.msg-partial .msg-content::after {
  content: ' ⏹ stopped';
  color: #d0a0a0;
  font-size: 12px;
}
.reasoning {
  margin-top: 8px;
  font-size: 12px;
  color: #9a9a9a;
}
.reasoning summary {
  cursor: pointer;
  user-select: none;
}
.reasoning-body {
  white-space: pre-wrap;
  margin-top: 6px;
  max-height: 220px;
  overflow-y: auto;
}
.tool-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 8px;
}
.tool-card {
  border: 1px solid #3a3a3a;
  border-radius: 8px;
  background: #1a1a1a;
  overflow: hidden;
}
.tool-head {
  display: flex;
  gap: 10px;
  align-items: center;
  width: 100%;
  padding: 8px 10px;
  background: none;
  border: none;
  color: inherit;
  cursor: pointer;
  text-align: left;
  font-size: 13px;
}
.tool-name {
  font-family: monospace;
  color: #7ec3ef;
  font-weight: 600;
}
.tool-args {
  flex: 1;
  color: #9a9a9a;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: 12px;
}
.tool-status {
  font-size: 11px;
  color: #b0b0b0;
  border: 1px solid #444;
  border-radius: 999px;
  padding: 2px 8px;
}
.tool-done .tool-status {
  color: #7fd18a;
  border-color: #2f6b3a;
}
.tool-error .tool-status,
.tool-denied .tool-status {
  color: #ef9a9a;
  border-color: #7a3a3a;
}
.tool-running .tool-status {
  color: #ffd479;
  border-color: #7a6230;
}
.tool-output {
  margin: 0;
  padding: 10px 12px;
  background: #0d0d0d;
  border-top: 1px solid #333;
  font-size: 12px;
  white-space: pre-wrap;
  overflow-x: auto;
  max-height: 320px;
}
.typing {
  display: flex;
  gap: 5px;
  padding: 6px 0;
}
.typing .dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #666;
  animation: pulse 1.2s infinite ease-in-out;
}
.typing .dot:nth-child(2) {
  animation-delay: 0.2s;
}
.typing .dot:nth-child(3) {
  animation-delay: 0.4s;
}
@keyframes pulse {
  0%, 80%, 100% { opacity: 0.25; transform: translateY(0); }
  40% { opacity: 1; transform: translateY(-3px); }
}
.approval {
  max-width: 860px;
  margin: 6px auto 18px;
  background: #2b2416;
  border: 1px solid #8a6d1f;
  border-radius: 10px;
  padding: 14px 16px;
}
.approval-title {
  font-weight: 600;
  margin-bottom: 6px;
}
.approval-title code {
  color: #7ec3ef;
}
.approval-args {
  display: block;
  font-size: 12px;
  color: #cfcfcf;
  background: #17130a;
  border-radius: 6px;
  padding: 8px 10px;
  overflow-x: auto;
  white-space: pre-wrap;
}
.approval-hint {
  margin: 8px 0 12px;
  font-size: 12px;
  color: #c9b78a;
}
.approval-actions {
  display: flex;
  gap: 10px;
}
.btn-allow,
.btn-deny,
.btn-send,
.btn-stop {
  border: none;
  border-radius: 8px;
  padding: 9px 18px;
  font-size: 14px;
  cursor: pointer;
}
.btn-allow {
  background: #2f7d3c;
  color: white;
}
.btn-deny {
  background: #4a3030;
  color: #f0c0c0;
}
.btn-send {
  background: #007acc;
  color: white;
}
.btn-send:disabled {
  background: #2a2a2a;
  color: #777;
  cursor: default;
}
.btn-stop {
  background: #7a2f2f;
  color: white;
}
.chat-error {
  max-width: 860px;
  margin: 0 auto 14px;
  color: #ffb4b4;
  background: #331d1d;
  border: 1px solid #7a3a3a;
  border-radius: 8px;
  padding: 10px 12px;
  font-size: 13px;
}
.composer {
  border-top: 1px solid #2c2c2c;
  background: #1b1b1b;
  padding: 10px 16px 14px;
}
.composer-meta {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: #8f8f8f;
  margin-bottom: 6px;
}
.working-hint {
  color: #ffd479;
}
.composer-row {
  display: flex;
  gap: 10px;
  align-items: flex-end;
}
.composer-input {
  flex: 1;
  resize: none;
  background: #101010;
  color: #e6e6e6;
  border: 1px solid #3a3a3a;
  border-radius: 10px;
  padding: 10px 12px;
  font-size: 14px;
  font-family: inherit;
  line-height: 1.4;
}
.composer-input:focus {
  outline: none;
  border-color: #007acc;
}
.composer-buttons {
  display: flex;
  flex-direction: column;
}
</style>
