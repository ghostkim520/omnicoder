export function generatePrompt(code, instruction) {
  return `You are an AI coding assistant. Complete or improve the following code based on the instruction:\n\nCode:\n${code}\n\nInstruction:\n${instruction}\n\nGenerated Code:\n`;
}

export function formatCodeOutput(output) {
  // Remove any leading/trailing whitespace or markers
  return output
    .replace(/^\s+|\s+$/g, '')
    .replace(/^```.*?\n|\n```.*?$/g, '')
    .trim();
}

export function checkProviderAvailability(provider) {
  // Mock function to check if a provider is available
  // In a real app, this would check network or local installation
  const availableProviders = ['ollama', 'lmstudio', 'gpt4all', 'openai'];
  return availableProviders.includes(provider);
}

export function getSuggestedModels() {
  // Return a list of suggested models for installation
  return [
    { name: 'llama3', provider: 'ollama', description: 'Advanced language model by Meta' },
    { name: 'mistral', provider: 'ollama', description: 'High-performance model by Mistral AI' },
    { name: 'stablelm-3b', provider: 'lmstudio', description: 'StableLM by Stability AI' },
    { name: 'wizardlm-7b', provider: 'gpt4all', description: 'WizardLM by The AI Lab' },
  ];
}