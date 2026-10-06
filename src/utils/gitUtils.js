import { invoke } from '@tauri-apps/api/core';

export async function commitChanges(message, files = ['.']) {
  try {
    await invoke('git_commit', { message });
    return { success: true, message: 'Changes committed successfully!' };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

export async function pushChanges() {
  try {
    await invoke('git_push');
    return { success: true, message: 'Changes pushed successfully!' };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

export async function checkGitStatus() {
  try {
    const status = await invoke('git_status');
    return { success: true, status };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

export async function runGitCommand(command) {
  try {
    const output = await invoke('run_command', { command });
    return { success: true, output };
  } catch (error) {
    return { success: false, error: error.message };
  }
}