<template>
  <div class="code-editor">
    <textarea v-model="code" placeholder="Type your code..." :class="{ 'error': error }" rows="15" cols="80"></textarea>
    <div class="actions">
      <button @click="generateCode">Generate Code</button>
      <button @click="debugCode" :disabled="!code.trim()">Debug</button>
      <button @click="commitChanges" :disabled="!code.trim()">Commit Changes</button>
    </div>
    <div v-if="error" class="error-message">{{ error }}</div>
    <div v-if="generatedCode" class="generated-code">
      <h3>Generated Code:</h3>
      <pre>{{ generatedCode }}</pre>
    </div>
    <div v-if="debugOutput" class="debug-output">
      <h3>Debug Output:</h3>
      <pre>{{ debugOutput }}</pre>
    </div>
  </div>
</template>

<script>
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useAIStore } from '../stores/aiStore';

export default {
  setup() {
    const aiStore = useAIStore();
    const code = ref('');
    const generatedCode = ref('');
    const debugOutput = ref('');
    const error = ref('');
    
    const generateCode = async () => {
      try {
        const response = await invoke('generate_code', {
          prompt: code.value,
          model: aiStore.selectedModel,
          useLocal: aiStore.settings.useLocal,
          maxTokens: aiStore.settings.maxTokens
        });
        if (response.error) {
          error.value = response.error;
          generatedCode.value = '';
        } else {
          generatedCode.value = response.code;
          error.value = '';
        }
      } catch (err) {
        error.value = err.message;
      }
    };

    
    const debugCode = async () => {
      try {
        debugOutput.value = await invoke('debug_code', { code: code.value });
      } catch (err) {
        debugOutput.value = `Error: ${err.message}`;
      }
    };
    
    const commitChanges = async () => {
      try {
        await invoke('git_commit', { message: `Auto-commit: Updated code` });
        await invoke('git_push');
        alert('Changes committed and pushed successfully!');
      } catch (err) {
        alert(`Error: ${err.message}`);
      }
    };
    
    return { code, generatedCode, debugOutput, error, generateCode, debugCode, commitChanges };
  }
};
</script>

<style>
.code-editor {
  font-family: 'Monaco', 'Menlo', monospace;
  margin-bottom: 20px;
}

textarea {
  font-family: inherit;
  padding: 10px;
  border: 1px solid #ccc;
  border-radius: 4px;
  font-size: 14px;
  width: 100%;
}

textarea.error {
  border-color: #ff6b6b;
}

.actions {
  margin: 10px 0;
  display: flex;
  gap: 10px;
}

button {
  padding: 8px 16px;
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

.error-message {
  color: #ff6b6b;
  margin: 10px 0;
}

.generated-code, .debug-output {
  margin-top: 20px;
  padding: 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
  background-color: #f9f9f9;
}

pre {
  white-space: pre-wrap;
  word-wrap: break-word;
  background-color: #f5f5f5;
  padding: 10px;
  border-radius: 4px;
}
</style>