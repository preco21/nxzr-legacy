import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';

export interface SubscribeLoggingResponse {
  logs: string[];
  task_name: string;
}

export class Logger {
  private initialized = false;
  private handle: string | undefined = undefined;
  private listeners: Set<(logs: string) => void> = new Set();
  public logs: string[] = [];

  constructor() {
    listen<SubscribeLoggingResponse>('logging:log', (event) => {
      const logString = event.payload as unknown as string;
      // FIXME: use concrete object
      this.logs.push(logString);
      for (const listener of this.listeners) {
        listener(logString);
      }
    });
  }

  public async init(): Promise<void> {
    if (this.initialized) {
      return;
    }
    const res = await invoke<SubscribeLoggingResponse>('subscribe_logging');
    this.logs = res.logs;
    this.handle = res.task_name;
    this.initialized = true;
  }

  public async info(message: string): Promise<void> {
    await invoke('log', { kind: 'info', message });
  }

  public async warn(message: string): Promise<void> {
    await invoke('log', { kind: 'warn', message });
  }

  public async error(message: string): Promise<void> {
    await invoke('log', { kind: 'error', message });
  }

  public onLog(listener: (logs: string) => void): (() => void) {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  public async dispose(): Promise<void> {
    if (!this.initialized) {
      return;
    }
    await invoke('cancel_task', { task_name: this.handle });
    this.logs = [];
    this.handle = undefined;
    this.initialized = false;
  }
}

export const logger = new Logger();
