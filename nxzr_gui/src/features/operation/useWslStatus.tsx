import { UnlistenFn, listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { isWslAnchorInstanceReady } from '../../common/commands';

export interface WslStatus {
  isReady: boolean;
}

export interface UseWslStatusOptions {
  onUpdate?: (status: WslStatus) => void;
}

export interface UseWslStatus {
  isReady: boolean;
}

export function useWslStatus(options?: UseWslStatusOptions): UseWslStatus {
  const [isReady, setIsReady] = useState(false);
  useEffect(() => {
    let unlisten: UnlistenFn;
    (async () => {
      unlisten = await listen('wsl:status_update', async () => {
        const ready = await isWslAnchorInstanceReady();
        setIsReady(ready);
        options?.onUpdate?.({ isReady: ready });
      });
    })();
    return () => unlisten?.();
  }, [options?.onUpdate]);
  return {
    isReady,
  };
}
