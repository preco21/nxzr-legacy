import { useCallback, useState } from 'react';
import { runAgentCheck } from '../../common/commands';

export interface UseAgent {
  pending: boolean;
  isReady: boolean;
  error?: Error;
  launchAgentDaemon: () => Promise<void>;
  shutdownAgentDaemon: () => Promise<void>;
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
  const launchAgentDaemon = useCallback(async () => {
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
  }, []);
  // FIXME: handle events: agent:status_change
  return {
    pending,
    isReady,
    error,
    launchAgentDaemon,
    shutdownAgentDaemon: async () => {},
  };
}
