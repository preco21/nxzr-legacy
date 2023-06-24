import { invoke } from '@tauri-apps/api/tauri';

// General
export async function windowReady(name: string): Promise<void> {
  await invoke('window_ready', { name });
}

export async function cancelTask(taskLabel: string): Promise<void> {
  await invoke('cancel_task', { taskLabel });
}

// Logging
export async function openLogWindow(): Promise<void> {
  await invoke('open_log_window');
}

export interface SubscribeLoggingResponse {
  logs: string[];
  task_label: string;
}

export async function subscribeLogging(): Promise<SubscribeLoggingResponse> {
  return invoke<SubscribeLoggingResponse>('subscribe_logging');
}

export async function sendLog(kind: 'info' | 'warn' | 'error', message: string): Promise<void> {
  await invoke('log', { kind, message });
}

export async function revealLogFolder(): Promise<void> {
  await invoke('reveal_log_folder');
}

// Setup
export async function check1SetupInstalled(): Promise<void> {
  await invoke('check_1_setup_installed');
}

export async function check2Wslconfig(): Promise<void> {
  await invoke('check_2_wslconfig');
}

export async function check3AgentRegistered(): Promise<void> {
  await invoke('check_3_agent_registered');
}

export async function install1ProgramSetup(): Promise<void> {
  await invoke('install_1_program_setup');
}

export async function install2EnsureWslconfig(): Promise<void> {
  await invoke('install_2_ensure_wslconfig');
}

export async function install3RegisterAgent(): Promise<void> {
  await invoke('install_3_register_agent');
}
