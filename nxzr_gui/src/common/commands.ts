import { invoke } from '@tauri-apps/api/tauri';

// General
export async function windowReady(name: string): Promise<void> {
  await wrapError(invoke('window_ready', { name }));
}

export async function cancelTask(taskLabel: string): Promise<void> {
  await wrapError(invoke('cancel_task', { taskLabel }));
}

// Logging
export async function openLogWindow(): Promise<void> {
  await wrapError(invoke('open_log_window'));
}

export interface SubscribeLoggingResponse {
  logs: string[];
  taskLabel: string;
}

export async function subscribeLogging(): Promise<SubscribeLoggingResponse> {
  return wrapError(invoke<SubscribeLoggingResponse>('subscribe_logging'));
}

export async function sendLog(kind: 'info' | 'warn' | 'error', message: string): Promise<void> {
  await wrapError(invoke('log', { kind, message }));
}

export async function revealLogFolder(): Promise<void> {
  await wrapError(invoke('reveal_log_folder'));
}

// Setup
export async function check1SetupInstalled(): Promise<void> {
  await wrapError(invoke('check_1_setup_installed'));
}

export async function check2Wslconfig(): Promise<void> {
  await wrapError(invoke('check_2_wslconfig'));
}

export async function check3AgentRegistered(): Promise<void> {
  await wrapError(invoke('check_3_agent_registered'));
}

export async function install1ProgramSetup(): Promise<void> {
  await wrapError(invoke('install_1_program_setup'));
}

export async function install2EnsureWslconfig(): Promise<void> {
  await wrapError(invoke('install_2_ensure_wslconfig'));
}

export async function install3RegisterAgent(): Promise<void> {
  await wrapError(invoke('install_3_register_agent'));
}

// Usbipd
export interface AdapterInfo {
  id: string;
  serial: string;
  name: string;
  busId: string;
  hardwareId: string;
  isAttached?: boolean;
}

export async function listHidAdapters(): Promise<AdapterInfo[]> {
  return wrapError(invoke<AdapterInfo[]>('list_hid_adapters'));
}

export async function attachHidAdapter(hardwareId: string): Promise<void> {
  await wrapError(invoke('attach_hid_adapter', { hardwareId }));
}

export async function detachHidAdapter(hardwareId: string): Promise<void> {
  await wrapError(invoke('detach_hid_adapter', { hardwareId }));
}

// WSL
export async function launchWslInstance(): Promise<void> {
  await wrapError(invoke('launch_wsl_instance'));
}

export async function runWslAgentCheck(): Promise<void> {
  await wrapError(invoke('run_wsl_agent_check'));
}

// Helpers
async function wrapError<T>(promise: Promise<T>): Promise<T> {
  try {
    return await promise;
  } catch (err) {
    if (err instanceof Error) {
      throw err;
    }
    throw new Error(err as string);
  }
}
