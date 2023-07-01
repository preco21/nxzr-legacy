import { useCallback, useState } from 'react';

export interface UseWslAnchorOptions {
  onFail?: (error: Error) => void;
}

export interface UseWslAnchor {
  pending?: boolean;
  error?: Error;
  launchAnchorInstance: () => Promise<void>;
}

export function useWslAnchor(options?: UseWslAnchorOptions): UseWslAnchor {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<Error | undefined>(undefined);
  const launchAnchorInstance = useCallback(async () => {
    try {
      setPending(true);
      await launchAnchorInstance();
    } catch (err) {
      setError(err as Error);
      options?.onFail?.(err as Error);
    } finally {
      setPending(false);
    }
  }, [options?.onFail]);
  return {
    pending,
    error,
    launchAnchorInstance,
  };
}
