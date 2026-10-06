<template>
  <div class="terminal-container">
    <div class="terminal-header">
      <h2>Terminal</h2>
      <div class="terminal-controls">
        <button @click="clearTerminal">Clear</button>
        <button @click="copyOutput">Copy</button>
      </div>
    </div>
    <div class="terminal-content" ref="terminalContent">
      <div v-for="(line, index) in terminalOutput" :key="index" class="terminal-line">
        {{ line }}
      </div>
      <div v-if="typingIndicator" class="typing-indicator">_</div>
    </div>
    <div class="terminal-input">
      <input
        v-model="inputCommand"
        @keyup.enter="executeCommand"
        placeholder="$ "
        ref="terminalInput"
      />
    </div>
  </div>
</template>

<script>
import { invoke } from '@tauri-apps/api/core';
import { ref, onMounted } from 'vue';

export default {
  setup() {
    const terminalOutput = ref([]);
    const inputCommand = ref('');
    const typingIndicator = ref(false);
    const terminalContent = ref(null);
    const terminalInput = ref(null);
    
    // Initialize terminal with a welcome message
    onMounted(() => {
      terminalOutput.value = [
        'Welcome to OmniCoder Terminal!',
        'Type commands to execute (e.g., ls, python --version)',
        '',
      ];
      scrollToBottom();
    });
    
    const executeCommand = async () => {
      if (!inputCommand.value.trim()) return;
      
      // Add user input to terminal
      terminalOutput.value.push(`$ ${inputCommand.value}`);
      scrollToBottom();
      
      // Simulate typing indicator
      typingIndicator.value = true;
      setTimeout(() => {
        typingIndicator.value = false;
      }, 1000);
      
      try {
        // Execute the command
        const result = await invoke('run_command', { command: inputCommand.value });
        
        // Add command output to terminal
        if (result.includes('Error:')) {
          terminalOutput.value.push(`Error: ${result.split('Error: ')[1]}`);
        } else {
          terminalOutput.value.push(result);
        }
        
        // Add a blank line for readability
        terminalOutput.value.push('');
      } catch (err) {
        terminalOutput.value.push(`Error: ${err.message}`);
        terminalOutput.value.push('');
      } finally {
        inputCommand.value = '';
        scrollToBottom();
      }
    };
    
    const clearTerminal = () => {
      terminalOutput.value = [];
      scrollToBottom();
    };
    
    const copyOutput = () => {
      const textToCopy = terminalOutput.value.join('\n');
      navigator.clipboard.writeText(textToCopy).then(() => {
        alert('Terminal output copied to clipboard!');
      });
    };
    
    const scrollToBottom = () => {
      if (terminalContent.value) {
        terminalContent.value.scrollTop = terminalContent.value.scrollHeight;
      }
    };
    
    return {
      terminalOutput,
      inputCommand,
      typingIndicator,
      terminalContent,
      terminalInput,
      executeCommand,
      clearTerminal,
      copyOutput,
    };
  },
};
</script>

<style>
.terminal-container {
  margin: 20px 0;
  padding: 15px;
  border: 1px solid #ddd;
  border-radius: 8px;
  background-color: #1e1e1e;
  color: #e0e0e0;
  font-family: 'Monaco', 'Menlo', monospace;
  height: 400px;
  display: flex;
  flex-direction: column;
}

.terminal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.terminal-header h2 {
  margin: 0;
  color: #e0e0e0;
}

.terminal-controls button {
  padding: 5px 10px;
  background-color: #3a3a3a;
  color: #e0e0e0;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  margin-left: 5px;
}

.terminal-content {
  flex: 1;
  overflow-y: auto;
  margin-bottom: 10px;
  padding: 10px;
  background-color: #1e1e1e;
}

.terminal-line {
  margin-bottom: 5px;
  white-space: pre-wrap;
}

.terminal-input {
  display: flex;
  padding: 5px;
  background-color: #252526;
  border-top: 1px solid #444;
}

.terminal-input input {
  flex: 1;
  background-color: #252526;
  color: #e0e0e0;
  border: none;
  padding: 5px;
  font-family: inherit;
  font-size: 14px;
}

.typing-indicator {
  color: #569cd6;
  animation: blink 1s infinite;
}

@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}
</style>