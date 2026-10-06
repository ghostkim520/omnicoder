import { defineStore } from 'pinia';

export const usePluginStore = defineStore('plugins', {
  state: () => ({
    plugins: [
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
    ],
  }),
  actions: {
    togglePlugin(name) {
      const plugin = this.plugins.find(p => p.name === name);
      if (plugin) {
        plugin.enabled = !plugin.enabled;
      }
    },
    enablePlugin(name) {
      const plugin = this.plugins.find(p => p.name === name);
      if (plugin) {
        plugin.enabled = true;
      }
    },
    disablePlugin(name) {
      const plugin = this.plugins.find(p => p.name === name);
      if (plugin) {
        plugin.enabled = false;
      }
    },
    addPlugin(plugin) {
      if (!this.plugins.some(p => p.name === plugin.name)) {
        this.plugins.push(plugin);
      }
    },
  },
})