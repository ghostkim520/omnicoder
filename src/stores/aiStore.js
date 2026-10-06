import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';

export const useAIStore = defineStore('ai', {
  state: () => ({
    activeProvider: 'omniroute',
    selectedModel: 'gemini/gemini-3.1-flash-lite',
    availableModels: [],
    settings: {
      temperature: 0.7,
      maxTokens: 4096,
      useLocal: false,
    },
    providers: {
      omniroute: {
        name: 'OmniRoute Gateway',
        online: false,
        description: 'Local AI gateway (localhost:20128) routing to 350+ providers.',
      },
      ollama: {
        name: 'Ollama',
        online: false,
        description: 'Local AI models (requires Ollama to be running)',
      },
      openai: {
        name: 'OpenAI',
        online: true,
        description: 'Cloud-based AI (requires API key)',
      },
    },
  }),
  actions: {
    async initialize() {
      await this.refreshStatus();
      await this.refreshModels();
    },
    async refreshStatus() {
      try {
        this.providers.omniroute.online = await invoke('gateway_status');
      } catch (e) {
        this.providers.omniroute.online = false;
      }
    },
    async refreshModels() {
      try {
        this.availableModels = await invoke('list_models');
        if (this.availableModels.length > 0 && !this.availableModels.includes(this.selectedModel)) {
          this.selectedModel = this.availableModels[0];
        }
      } catch (e) {
        this.availableModels = [];
      }
    },
    setProvider(provider) {
      this.activeProvider = provider;
    },
    setModel(model) {
      this.selectedModel = model;
    },
    updateSetting(key, value) {
      this.settings[key] = value;
    },
  },
})
