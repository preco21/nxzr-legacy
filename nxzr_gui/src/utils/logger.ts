import { invoke } from '@tauri-apps/api/tauri';
import { UnlistenFn, listen } from '@tauri-apps/api/event';
import { appWindow } from '@tauri-apps/api/window';

export interface LogEntry {
  timestamp: string;
  level: 'ERROR' | 'WARN' | 'INFO' | 'DEBUG' | 'TRACE';
  fields: {
    message: string;
  };
  target: string;
}

export interface SubscribeLoggingResponse {
  logs: string[];
  task_name: string;
}

export class LogListener {
  private state: 'init' | 'pending' | 'ready' = 'init';
  private taskName: string | undefined = undefined;
  private listeners: Set<(logs: string) => void> = new Set();
  private internalLoggerHandle: UnlistenFn | undefined = undefined;
  public initialLogs: string[] = [];

  public async init(): Promise<void> {
    if (this.state !== 'init') {
      return;
    }
    this.state = 'pending';
    this.internalLoggerHandle = await listen<SubscribeLoggingResponse>('logging:log', (event) => {
      const logString = event.payload as unknown as string;
      // FIXME: use concrete object
      for (const listener of this.listeners) {
        listener(logString);
      }
      this.initialLogs.push(logString);
    });
    const res = await invoke<SubscribeLoggingResponse>('subscribe_logging');
    this.initialLogs = res.logs;
    this.taskName = res.task_name;
    this.state = 'ready';
  }

  public onLog(listener: (logs: string) => void): (() => void) {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  public async dispose(): Promise<void> {
    if (this.state !== 'ready') {
      return;
    }
    this.internalLoggerHandle?.();
    await invoke('cancel_task', { taskName: this.taskName });
    this.initialLogs = [];
    this.taskName = undefined;
    this.listeners.clear();
    this.state = 'init';
  }
}

export const logListener = new LogListener();

export async function info(message: string): Promise<void> {
  await invoke('log', { kind: 'info', message });
}

export async function warn(message: string): Promise<void> {
  await invoke('log', { kind: 'warn', message });
}

export async function error(message: string): Promise<void> {
  await invoke('log', { kind: 'error', message });
}

export async function setupListenerHook(): Promise<void> {
  await logListener.init();
  appWindow.once('tauri://close-requested', async () => {
    await logListener.dispose();
    appWindow.close();
  });
  window.addEventListener('beforeunload', () => {
    logListener.dispose();
  });
}
