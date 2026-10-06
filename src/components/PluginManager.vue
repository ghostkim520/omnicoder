<template>
  <div class="plugin-manager">
    <h2>Plugins</h2>
    <div class="plugin-list">
      <div v-for="plugin in plugins" :key="plugin.name" class="plugin-card">
        <h3>{{ plugin.name }} <span v-if="plugin.enabled" class="enabled-badge">Enabled</span></h3>
        <p>Version: {{ plugin.version }}</p>
        <div class="plugin-actions">
          <button @click="togglePlugin(plugin.name)" :disabled="plugin.enabled === toggleState">
            {{ plugin.enabled ? 'Disable' : 'Enable' }}
          </button>
          <button @click="installPlugin(plugin.name)" v-if="!plugin.enabled">Install</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import { invoke } from '@tauri-apps/api/core';
import { ref, onMounted } from 'vue';

export default {
  setup() {
    const plugins = ref([]);
    const toggleState = ref(false);
    
    onMounted(async () => {
      // Load plugins from the backend
      plugins.value = await loadPlugins();
    });
    
    const loadPlugins = async () => {
      // Mock data for now; in a real app, this would come from the backend
      return [
        {
          name: "github-copilot",
          version: "1.0",
          commands: ["copilot_suggest"],
          enabled: false,
        },
        {
          name: "debugger",
          version: "1.0",
          commands: ["debug_code"],
          enabled: true,
        },
        {
          name: "terminal",
          version: "1.0",
          commands: ["run_command"],
          enabled: true,
        },
        {
          name: "git-integration",
          version: "1.0",
          commands: ["git_commit", "git_push"],
          enabled: true,
        },
      ];
    };
    
    const togglePlugin = async (name) => {
      toggleState.value = true;
      try {
        // In a real app, this would update the backend state
        const plugin = plugins.value.find(p => p.name === name);
        if (plugin) {
          plugin.enabled = !plugin.enabled;
        }
        alert(`${name} ${plugin.enabled ? 'enabled' : 'disabled'} successfully!`);
      } catch (err) {
        alert(`Error: ${err.message}`);
      } finally {
        toggleState.value = false;
      }
    };
    
    const installPlugin = async (name) => {
      try {
        // In a real app, this would trigger an installation process
        alert(`Installing ${name}...`);
        // Example: For Ollama, this could run `ollama pull <model>`
        if (name === "github-copilot") {
          alert("GitHub Copilot requires a separate setup. Please visit https://github.com/features/copilot.");
        }
      } catch (err) {
        alert(`Error: ${err.message}`);
      }
    };
    
    return { plugins, togglePlugin, installPlugin, toggleState };
  },
};
</script>

<style>
.plugin-manager {
  margin: 20px 0;
  padding: 20px;
  border: 1px solid #ddd;
  border-radius: 8px;
  background-color: #f9f9f9;
}

.plugin-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 20px;
}

.plugin-card {
  padding: 15px;
  border: 1px solid #eee;
  border-radius: 6px;
  background-color: white;
}

.plugin-card h3 {
  margin-top: 0;
  display: flex;
  align-items: center;
  gap: 10px;
}

.enabled-badge {
  padding: 2px 8px;
  background-color: #4CAF50;
  color: white;
  border-radius: 4px;
  font-size: 12px;
}

.plugin-actions {
  margin-top: 15px;
  display: flex;
  gap: 10px;
}

button {
  padding: 8px 12px;
  background-color: #4CAF50;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

button:disabled {
  background-color: #cccccc;
  cursor: not-allowed;
}
</style>