import { invoke } from '@tauri-apps/api/tauri';
import { UnlistenFn, listen } from '@tauri-apps/api/event';
import { appWindow } from '@tauri-apps/api/window';

export type LogLevel = 'ERROR' | 'WARN' | 'INFO' | 'DEBUG' | 'TRACE';

export interface LogEntry {
  id: number;
  timestamp: string;
  level: LogLevel;
  fields: {
    message: string;
  };
  target: string;
}

export interface SubscribeLoggingResponse {
  logs: string[];
  task_label: string;
}

export class LogListener {
  private state: 'init' | 'pending' | 'ready' = 'init';
  private taskLabel: string | undefined = undefined;
  private listeners: Set<(entry: LogEntry) => void> = new Set();
  private internalLoggerHandle: UnlistenFn | undefined = undefined;
  private logId: number = 0;
  public initialLogs: LogEntry[] = [];

  public async init(): Promise<void> {
    if (this.state !== 'init') {
      return;
    }
    this.state = 'pending';
    this.internalLoggerHandle = await listen<SubscribeLoggingResponse>('logging:log', (event) => {
      const logString = event.payload as unknown as string;
      const parsed = { ...JSON.parse(logString) as LogEntry, id: this.logId++ };
      for (const listener of this.listeners) {
        listener(parsed);
      }
      this.initialLogs.push(parsed);
    });
    const res = await invoke<SubscribeLoggingResponse>('subscribe_logging');
    this.initialLogs = res.logs.map((logString) => ({
      ...JSON.parse(logString) as LogEntry,
      id: this.logId++,
    }));
    this.taskLabel = res.task_label;
    this.state = 'ready';
  }

  public onLog(listener: (entry: LogEntry) => void): (() => void) {
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
    await invoke('cancel_task', { taskLabel: this.taskLabel });
    this.initialLogs = [];
    this.taskLabel = undefined;
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
