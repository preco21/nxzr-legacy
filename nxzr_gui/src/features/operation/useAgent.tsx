import { useCallback, useEffect, useState } from 'react';
import { isAgentDaemonReady, launchAgentDaemon, runAgentCheck, terminateAgentDaemon } from '../../common/commands';
import { WslStatus, useWslStatus } from './useWslStatus';
import { UnlistenFn, listen } from '@tauri-apps/api/event';

export interface UseAgent {
  pending: boolean;
  isReady: boolean;
  error?: Error;
  launchDaemon: () => Promise<void>;
  terminateDaemon: () => Promise<void>;
}

export interface UseAgentOptions {
  onLaunchFailed?: (error: Error) => void;
  // FIXME:
  // onCheckFailed?: (error: Error) => void;
}

export function useAgent(options?: UseAgentOptions): UseAgent {
  const [isReady, setIsReady] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<Error | undefined>(undefined);
  const launchDaemon = useCallback(async () => {
    if (pending) {
      return;
    }
    try {
      setPending(true);
      // TODO: we can fallback to restarting wsl instead of restarting the entire app.
      await runAgentCheck();
      await launchAgentDaemon();
      setIsReady(true);
    } catch (err) {
      setError(err as Error);
      options?.onLaunchFailed?.(err as Error);
    } finally {
      setPending(false);
    }
  }, [pending]);
  const terminateDaemon = useCallback(async () => {
    if (pending) {
      return;
    }
    try {
      setPending(true);
      await terminateAgentDaemon();
      setIsReady(false);
    } catch (err) {
      setError(err as Error);
    } finally {
      setPending(false);
    }
  }, []);
  useWslStatus({
    onUpdate: useCallback((status: WslStatus) => {
      if (!status.isReady) {
        setIsReady(false);
      }
    }, []),
  });
  useEffect(() => {
    let unlisten: UnlistenFn;
    (async () => {
      unlisten = await listen('agent:status_update', async () => {
        const ready = await isAgentDaemonReady();
        setIsReady(ready);
      });
    })();
    return () => unlisten?.();
  }, []);
  return {
    pending,
    isReady,
    error,
    launchDaemon,
    terminateDaemon,
  };
}
