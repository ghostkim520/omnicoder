<template>
  <div class="app-container">
    <header>
      <div class="brand">
        <h1>OmniCoder</h1>
        <p>A cross-platform AI coding assistant</p>
      </div>
      <div class="header-controls">
        <span
          class="gateway-pill"
          :class="ai.providers.omniroute.online ? 'on' : 'off'"
        >
          {{ ai.providers.omniroute.online ? 'Gateway online' : 'Gateway offline' }}
        </span>
        <select
          class="model-select"
          v-model="ai.selectedModel"
          aria-label="Model"
        >
          <option
            v-for="m in ai.availableModels"
            :key="m"
            :value="m"
          >
            {{ m }}
          </option>
          <option
            v-if="ai.availableModels.length && !ai.availableModels.includes(ai.selectedModel)"
            :value="ai.selectedModel"
          >
            {{ ai.selectedModel }}
          </option>
        </select>
        <select
          class="access-select"
          :value="chat.pcAccess"
          aria-label="PC access mode"
          title="How much can the agent do on this PC without asking"
          @change="chat.setAccess($event.target.value)"
        >
          <option value="readonly">Read-only</option>
          <option value="ask">Ask before changes</option>
          <option value="auto">Full auto</option>
        </select>
      </div>
    </header>

    <div class="main-content">
      <div class="sidebar">
        <button class="new-chat" @click="chat.newChat()">+ New chat</button>
        <div class="chat-list">
          <button
            v-for="c in chat.conversations"
            :key="c.id"
            class="chat-item"
            :class="{ active: c.id === chat.activeId }"
            @click="chat.selectChat(c.id)"
          >
            <span class="chat-item-title">{{ c.title }}</span>
            <span class="chat-item-count">{{ c.messages.length }}</span>
          </button>
        </div>
      </div>

      <div class="editor-area">
        <nav class="tabs">
          <button
            v-for="t in tabs"
            :key="t.id"
            class="tab"
            :class="{ active: tab === t.id }"
            @click="tab = t.id"
          >
            {{ t.label }}
          </button>
        </nav>

        <div class="panels">
          <div class="panel panel-chat" v-show="tab === 'chat'">
            <ChatView />
          </div>
          <div class="panel panel-scroll" v-show="tab === 'code'">
            <CodeEditor />
          </div>
          <div class="panel panel-scroll" v-show="tab === 'terminal'">
            <Terminal />
          </div>
          <div class="panel panel-scroll" v-show="tab === 'providers'">
            <AIProviderSelector />
          </div>
          <div class="panel panel-scroll" v-show="tab === 'plugins'">
            <PluginManager />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import { ref, onMounted } from 'vue';
import AIProviderSelector from './components/AIProviderSelector.vue';
import PluginManager from './components/PluginManager.vue';
import CodeEditor from './components/CodeEditor.vue';
import Terminal from './components/Terminal.vue';
import ChatView from './components/ChatView.vue';
import { useAIStore } from './stores/aiStore';
import { useChatStore } from './stores/chatStore';

const TABS = [
  { id: 'chat', label: 'Chat' },
  { id: 'code', label: 'Code' },
  { id: 'terminal', label: 'Terminal' },
  { id: 'providers', label: 'Providers' },
  { id: 'plugins', label: 'Plugins' },
];

export default {
  components: {
    AIProviderSelector,
    PluginManager,
    CodeEditor,
    Terminal,
    ChatView,
  },
  setup() {
    const ai = useAIStore();
    const chat = useChatStore();
    const tab = ref('chat');
    onMounted(() => {
      console.log('OmniCoder loaded successfully!');
    });
    return { ai, chat, tab, tabs: TABS };
  },
};
</script>

<style>
body {
  font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  margin: 0;
  padding: 0;
  background-color: #161616;
  color: #e6e6e6;
}

.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  background-color: #2c3e50;
  color: white;
  padding: 10px 16px;
  flex: none;
}

header .brand h1 {
  margin: 0;
  font-size: 20px;
}

header .brand p {
  margin: 2px 0 0;
  font-size: 12px;
  opacity: 0.9;
}

header .header-controls {
  display: flex;
  align-items: center;
  gap: 10px;
}

.gateway-pill {
  font-size: 12px;
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid #555;
}
.gateway-pill.on {
  background: #1e3b24;
  border-color: #2f7d3c;
  color: #9fe0ac;
}
.gateway-pill.off {
  background: #3b1e1e;
  border-color: #7a3a3a;
  color: #ffb4b4;
}

.model-select,
.access-select {
  background: #1f2b38;
  color: #e6e6e6;
  border: 1px solid #4a5b6d;
  border-radius: 6px;
  padding: 6px 8px;
  font-size: 13px;
}

.main-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.sidebar {
  width: 260px;
  flex: none;
  background-color: #1b1b1b;
  border-right: 1px solid #2c2c2c;
  overflow-y: auto;
  padding: 12px;
}

.new-chat {
  width: 100%;
  padding: 9px 12px;
  background: #007acc;
  color: white;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  cursor: pointer;
}
.new-chat:hover {
  background: #0088e0;
}

.chat-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 12px;
}

.chat-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  background: transparent;
  color: #cfcfcf;
  border: 1px solid transparent;
  border-radius: 8px;
  cursor: pointer;
  text-align: left;
  font-size: 13px;
}
.chat-item:hover {
  background: #242424;
}
.chat-item.active {
  background: #1d3a52;
  border-color: #2c5f8a;
}
.chat-item-title {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.chat-item-count {
  font-size: 11px;
  color: #8f8f8f;
}

.editor-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.tabs {
  display: flex;
  gap: 2px;
  background: #1b1b1b;
  border-bottom: 1px solid #2c2c2c;
  padding: 6px 8px 0;
  flex: none;
}

.tab {
  padding: 8px 16px;
  background: transparent;
  color: #a9a9a9;
  border: 1px solid transparent;
  border-bottom: none;
  border-radius: 8px 8px 0 0;
  cursor: pointer;
  font-size: 13px;
}
.tab:hover {
  color: #e6e6e6;
}
.tab.active {
  background: #161616;
  color: #ffffff;
  border-color: #2c2c2c;
  position: relative;
  top: 1px;
}

.panels {
  flex: 1;
  display: flex;
  min-height: 0;
  background: #161616;
}

.panel {
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.panel-chat {
  display: flex;
  flex-direction: column;
}
.panel-chat > * {
  flex: 1;
  min-height: 0;
}

.panel-scroll {
  overflow-y: auto;
  background: #f5f5f5;
}
</style>
