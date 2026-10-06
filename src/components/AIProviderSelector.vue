<template>
  <div class="ai-provider-selector">
    <h2>AI Providers</h2>
    <div class="provider-options">
      <div 
        v-for="(provider, id) in aiStore.providers" 
        :key="id"
        :class="['provider-card', { active: aiStore.activeProvider === id }]"
      >
        <div class="provider-header">
          <h3>{{ provider.name }}</h3>
          <span :class="provider.online ? 'status-online' : 'status-offline'">
            {{ provider.online ? 'Online' : 'Offline' }}
          </span>
        </div>
        <p>{{ provider.description }}</p>
        <button 
          @click="aiStore.setProvider(id)"
          :disabled="aiStore.activeProvider === id"
        >
          {{ aiStore.activeProvider === id ? 'Selected' : 'Use Provider' }}
        </button>
      </div>
    </div>

    <div v-if="aiStore.availableModels.length" class="provider-suggestions">
      <h3>🔄 Available Models (OmniRoute)</h3>
      <p class="model-hint">
        {{ aiStore.availableModels.length }} models available — pick the active one with
        the model dropdown in the header.
      </p>
      <ul class="model-list">
        <li
          v-for="model in aiStore.availableModels"
          :key="model"
          :class="{ current: model === aiStore.selectedModel }"
        >
          {{ model }}
        </li>
      </ul>
    </div>
    <div v-else class="provider-suggestions">
      <p>No models found on OmniRoute gateway. Check if it's running on port 20128.</p>
      <button @click="aiStore.refreshModels()">Retry Fetch</button>
    </div>
  </div>
</template>

<script>
import { useAIStore } from '../stores/aiStore';
import { onMounted } from 'vue';

export default {
  setup() {
    const aiStore = useAIStore();

    onMounted(async () => {
      await aiStore.initialize();
    });

    return { aiStore };
  }
};
</script>

<style scoped>
.model-hint {
  font-size: 0.85em;
  opacity: 0.85;
}
.model-list {
  margin: 8px 0 0;
  padding-left: 18px;
  font-size: 0.85em;
}
.model-list .current {
  color: #4caf50;
  font-weight: 600;
}
.ai-provider-selector {
  padding: 20px;
  background: #1e1e1e;
  color: white;
  height: 100%;
  overflow-y: auto;
}
.provider-options {
  display: grid;
  gap: 15px;
  margin-bottom: 20px;
}
.provider-card {
  padding: 15px;
  border: 1px solid #333;
  border-radius: 8px;
  background: #252525;
}
.provider-card.active {
  border-color: #007acc;
  background: #1a2a3a;
}
.provider-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.status-online { color: #4caf50; font-size: 0.8em; }
.status-offline { color: #f44336; font-size: 0.8em; }
button {
  margin-top: 10px;
  padding: 8px 16px;
  background: #007acc;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}
button:disabled { background: #333; cursor: default; }
</style>
