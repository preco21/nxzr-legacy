import { useCallback, useMemo, useState } from 'react';
import { AdapterInfo, attachHidAdapter, detachHidAdapter, listHidAdapters } from '../../common/commands';

export interface UseAdapterManagerOptions {
  onAdapterLost?: (adapter: AdapterInfo) => void;
  onAdapterAutoSelected?: (adapter: AdapterInfo) => void;
  onAttached?: (adapter: AdapterInfo) => void;
  onDetached?: (adapter: AdapterInfo) => void;
}

export interface UseAdapterManager {
  pending: boolean;
  adapters: AdapterInfo[];
  selectedAdapter?: AdapterInfo;
  refreshAdapterList: () => Promise<void>;
  attachAdapter: (id: string) => Promise<void>;
  detachAdapter: (id: string) => Promise<void>;
}

export function useAdapterManager(options?: UseAdapterManagerOptions): UseAdapterManager {
  const [pending, setPending] = useState<boolean>(false);
  const [currentAdapterId, setCurrentAdapterId] = useState<string | undefined>(undefined);
  const [adapters, setAdapters] = useState<AdapterInfo[]>([]);
  const selectedAdapter = useMemo(() => {
    return adapters.find((adapter) => adapter.id === currentAdapterId);
  }, [adapters, currentAdapterId]);
  const refreshAdapterList = useCallback(async () => {
    try {
      setPending(true);
      setAdapters([]);
      const newAdapters = await listHidAdapters();
      // Check if the current adapter is still available.
      if (selectedAdapter != null) {
        const targetAdapter = newAdapters.find((adapter) => {
          return adapter.id === selectedAdapter.id;
        });
        if (targetAdapter == null) {
          setCurrentAdapterId(undefined);
          options?.onAdapterLost?.(selectedAdapter);
        }
      } else {
        // Infer the current adapter from the list.
        const currentlyAttached = newAdapters.find((adapter) => adapter.isAttached);
        if (currentlyAttached != null) {
          setCurrentAdapterId(currentlyAttached.id);
          options?.onAdapterAutoSelected?.(currentlyAttached);
        }
      }
      setAdapters(newAdapters);
    } finally {
      setPending(false);
    }
  }, [selectedAdapter]);
  const attachAdapter = useCallback(async (id: string) => {
    const targetAdapter = adapters.find((adapter) => adapter.id === id);
    if (targetAdapter == null) {
      return;
    }
    try {
      setPending(true);
      await attachHidAdapter(targetAdapter.hardwareId);
      const newAdapters = await listHidAdapters();
      setAdapters(newAdapters);
      setCurrentAdapterId(targetAdapter.id);
      options?.onAttached?.(targetAdapter);
    } catch (err) {
      setCurrentAdapterId(undefined);
      throw err;
    } finally {
      setPending(false);
    }
  }, [adapters]);
  const detachAdapter = useCallback(async (id: string) => {
    const targetAdapter = adapters.find((adapter) => adapter.id === id);
    if (targetAdapter == null) {
      return;
    }
    try {
      setPending(true);
      await detachHidAdapter(targetAdapter.hardwareId);
      const newAdapters = await listHidAdapters();
      setAdapters(newAdapters);
      options?.onDetached?.(targetAdapter);
    } finally {
      setPending(false);
    }
  }, []);
  return {
    pending,
    adapters,
    selectedAdapter,
    refreshAdapterList,
    attachAdapter,
    detachAdapter,
  };
}
