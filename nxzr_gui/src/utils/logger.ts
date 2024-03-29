import { UnlistenFn, listen } from '@tauri-apps/api/event';
import { appWindow } from '@tauri-apps/api/window';
import { SubscribeLoggingResponse, sendLog, subscribeLogging, unsubscribeLogging } from '../common/commands';

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

export class LoggingSub {
  private state: 'init' | 'pending' | 'ready' = 'init';
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
    const res = await subscribeLogging();
    this.initialLogs = res.logs.map((logString) => ({
      ...JSON.parse(logString) as LogEntry,
      id: this.logId++,
    }));
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
    await unsubscribeLogging();
    this.initialLogs = [];
    this.listeners.clear();
    this.state = 'init';
  }
}

export const loggingSub = new LoggingSub();

export async function info(message: string): Promise<void> {
  await sendLog('info', message);
}

export async function warn(message: string): Promise<void> {
  await sendLog('warn', message);
}

export async function error(message: string): Promise<void> {
  await sendLog('error', message);
}

export async function setupListenerHook(): Promise<void> {
  await loggingSub.init();
  appWindow.once('tauri://close-requested', async () => {
    await loggingSub.dispose();
    appWindow.close();
  });
  window.addEventListener('beforeunload', () => {
    loggingSub.dispose();
  });
}
