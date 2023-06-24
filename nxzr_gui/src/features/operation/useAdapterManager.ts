import { useCallback, useEffect, useMemo, useState } from 'react';
import { attachHidAdapter, detachHidAdapter, listHidAdapters } from '../../common/commands';

export interface HidAdapter {
  id: string;
  name: string;
  busId: string;
  hardwareId: string;
  isAttached?: boolean;
}

export interface UseAdapterManagerOptions {
  enabled?: boolean;
  onAdapterLost?: (adapter: HidAdapter) => void;
  onAttached?: (adapter: HidAdapter) => void;
  onDetached?: (adapter: HidAdapter) => void;
}

export interface UseAdapterManager {
  pending: boolean;
  error?: Error;
  adapters: HidAdapter[];
  selectedAdapter?: HidAdapter;
  refreshAdapterList: () => void;
  attachAdapter: (id: string) => void;
  detachAdapter: (id: string) => void;
}

export function useAdapterManager(options?: UseAdapterManagerOptions): UseAdapterManager {
  const [pending, setPending] = useState<boolean>(false);
  const [error, setError] = useState<Error | undefined>(undefined);
  const [currentAdapterId, setCurrentAdapterId] = useState<string | undefined>(undefined);
  const [adapters, setAdapters] = useState<HidAdapter[]>([]);
  const selectedAdapter = useMemo(() => {
    return adapters.find((adapter) => adapter.id === currentAdapterId);
  }, [adapters, currentAdapterId]);
  const refreshAdapterList = useCallback(async () => {
    try {
      setPending(true);
      setAdapters([]);
      const adapterInfoList = await listHidAdapters();
      const formattedAdapters = adapterInfoList.map((adapter) => ({
        id: adapter.id,
        name: adapter.name,
        busId: adapter.bus_id,
        hardwareId: adapter.hardware_id,
        isAttached: adapter.is_attached,
      }));
      // Check if the current adapter is still available.
      if (selectedAdapter != null) {
        const targetAdapter = adapters.find((adapter) => adapter.id === selectedAdapter.id);
        if (targetAdapter == null) {
          setCurrentAdapterId(undefined);
          options?.onAdapterLost?.(selectedAdapter);
        }
      }
      setAdapters(formattedAdapters);
    } catch (err) {
      if (err instanceof Error) {
        setError(err);
      } else {
        setError(new Error(err as string));
      }
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
      options?.onAttached?.(targetAdapter);
    } catch (err) {
      if (err instanceof Error) {
        setError(err);
      } else {
        setError(new Error(err as string));
      }
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
      options?.onDetached?.(targetAdapter);
    } finally {
      setPending(false);
    }
  }, []);
  useEffect(() => {
    if (options?.enabled) {
      refreshAdapterList();
    }
  }, [options?.enabled, refreshAdapterList]);
  return {
    pending,
    error,
    adapters,
    selectedAdapter,
    refreshAdapterList,
    attachAdapter,
    detachAdapter,
  };
}
