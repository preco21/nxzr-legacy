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
  const handleAdapterUpdate = useCallback((
    newAdapters: AdapterInfo[],
    currentAdapter: AdapterInfo | undefined,
  ) => {
    // Check if the current adapter is still available.
    if (currentAdapter != null) {
      const targetAdapter = newAdapters.find((adapter) => {
        return adapter.id === currentAdapter.id;
      });
      if (targetAdapter == null || !targetAdapter.isAttached) {
        setCurrentAdapterId(undefined);
        options?.onAdapterLost?.(currentAdapter);
      }
    } else {
      // Infer the current adapter from the list.
      const currentlyAttached = newAdapters.find((adapter) => adapter.isAttached);
      if (currentlyAttached != null) {
        setCurrentAdapterId(currentlyAttached.id);
        options?.onAdapterAutoSelected?.(currentlyAttached);
      }
    }
  }, [options?.onAdapterLost, options?.onAdapterAutoSelected]);
  const refreshAdapterList = useCallback(async () => {
    try {
      setPending(true);
      const newAdapters = await listHidAdapters();
      handleAdapterUpdate(newAdapters, selectedAdapter);
      setAdapters(newAdapters);
    } catch (err) {
      setAdapters([]);
      throw err;
    } finally {
      setPending(false);
    }
  }, [selectedAdapter, handleAdapterUpdate]);
  const attachAdapter = useCallback(async (id: string) => {
    const targetAdapter = adapters.find((adapter) => adapter.id === id);
    if (targetAdapter == null) {
      return;
    }
    try {
      setPending(true);
      await attachHidAdapter(targetAdapter.hardwareId);
      // Fetch new adapter state.
      const newAdapters = await listHidAdapters();
      setAdapters(newAdapters);
      setCurrentAdapterId(targetAdapter.id);
      // Check if the `targetAdapter` is still available.
      handleAdapterUpdate(newAdapters, targetAdapter);
      options?.onAttached?.(targetAdapter);
    } catch (err) {
      setCurrentAdapterId(undefined);
      throw err;
    } finally {
      setPending(false);
    }
  }, [adapters, handleAdapterUpdate, options?.onAttached]);
  const detachAdapter = useCallback(async (id: string) => {
    const targetAdapter = adapters.find((adapter) => adapter.id === id);
    if (targetAdapter == null) {
      return;
    }
    try {
      setPending(true);
      await detachHidAdapter(targetAdapter.hardwareId);
      // Fetch new adapter state.
      const newAdapters = await listHidAdapters();
      setAdapters(newAdapters);
      setCurrentAdapterId(undefined);
      options?.onDetached?.(targetAdapter);
    } finally {
      setPending(false);
    }
  }, [options?.onDetached]);
  return {
    pending,
    adapters,
    selectedAdapter,
    refreshAdapterList,
    attachAdapter,
    detachAdapter,
  };
}
