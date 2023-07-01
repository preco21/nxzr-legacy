import { useCallback, useState } from 'react';
import { launchWslAnchorInstance } from '../../common/commands';
import { WslStatus, useWslStatus } from './useWslStatus';

export interface UseWslAnchorOptions {
  onFailure?: (error: Error) => void;
}

export interface UseWslAnchor {
  pending: boolean;
  isReady: boolean;
  error?: Error;
  launchAnchorInstance: () => Promise<void>;
}

export function useWslAnchor(options?: UseWslAnchorOptions): UseWslAnchor {
  const [isReady, setIsReady] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<Error | undefined>(undefined);
  useWslStatus({
    onUpdate: useCallback((status: WslStatus) => {
      if (!status.isReady) {
        setIsReady(false);
      }
    }, []),
  });
  const launchAnchorInstance = useCallback(async () => {
    if (pending) {
      return;
    }
    try {
      setPending(true);
      await launchWslAnchorInstance();
      setIsReady(true);
    } catch (err) {
      setError(err as Error);
      options?.onFailure?.(err as Error);
    } finally {
      setPending(false);
    }
  }, [pending, options?.onFailure]);
  return {
    pending,
    isReady,
    error,
    launchAnchorInstance,
  };
}
