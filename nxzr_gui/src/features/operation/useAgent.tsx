import { useCallback, useEffect, useState } from 'react';
import {
  RpcGetDeviceStatusResponse,
  isAgentDaemonReady,
  launchAgentDaemon,
  rpcConnectSwitch,
  rpcGetDeviceStatus,
  rpcRunControlStream,
  runAgentCheck,
  terminateAgentDaemon,
} from '../../common/commands';
import { WslStatus, useWslStatus } from './useWslStatus';
import { UnlistenFn, emit, listen } from '@tauri-apps/api/event';


// @ts-ignore
window.tauriemit = emit;

export interface UseAgentOptions {
  onLaunchFailed?: (error: Error) => void;
  // FIXME:
  // onCheckFailed?: (error: Error) => void;
}

export interface UseAgent {
  pending: boolean;
  isReady: boolean;
  error?: Error;
  deviceStatus?: RpcGetDeviceStatusResponse;
  switchConnected: boolean;
  inControlMode: boolean;
  launchDaemon: () => Promise<void>;
  terminateDaemon: () => Promise<void>;
  connectSwitch: () => Promise<void>;
}

export function useAgent(options?: UseAgentOptions): UseAgent {
  const [isReady, setIsReady] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<Error | undefined>(undefined);
  const [
    deviceStatus,
    setDeviceStatus,
  ] = useState<RpcGetDeviceStatusResponse | undefined>(undefined);
  const [switchConnected, setSwitchConnected] = useState(false);
  const [inControlMode, setInControlMode] = useState(false);
  const launchDaemon = useCallback(async () => {
    if (pending) {
      return;
    }
    try {
      setPending(true);
      // TODO: we can fallback to restarting wsl instead of restarting the entire app.
      await runAgentCheck();
      await launchAgentDaemon();
      const devStatus = await rpcGetDeviceStatus();
      setDeviceStatus(devStatus);
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
      setDeviceStatus(undefined);
      setIsReady(false);
    } catch (err) {
      setError(err as Error);
    } finally {
      setPending(false);
    }
  }, []);
  const connectSwitch = useCallback(async () => {
    if (pending) {
      return;
    }
    try {
      setPending(true);
      await rpcConnectSwitch();
      setSwitchConnected(true);
      setInControlMode(true);

      // FIXME: move this logic to separate hook.
      await rpcRunControlStream();
      emit('control:input', {
        messageFoo: 123,
        bar: {
          f: 123,
        },
      });
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
    deviceStatus,
    switchConnected,
    inControlMode,
    launchDaemon,
    terminateDaemon,
    connectSwitch,
  };
}
