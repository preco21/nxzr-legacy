import { UnlistenFn, emit, listen } from '@tauri-apps/api/event';
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
import { ControllerConfig, ControllerEventManager } from '../control/controller';

const CTRL_CONFIG = {
  button: [
    {
      keyboardKey: '1',
      button: 'Left',
    },
    {
      keyboardKey: '2',
      button: 'Up',
    },
    {
      keyboardKey: '3',
      button: 'Down',
    },
    {
      keyboardKey: '4',
      button: 'Right',
    },
    {
      keyboardKey: 'Tab',
      button: 'X',
    },
    {
      keyboardKey: 'q',
      button: 'RStick',
    },
    {
      keyboardKey: 'e',
      button: 'Minus',
    },
    {
      keyboardKey: 'r',
      button: 'Plus',
    },
    {
      keyboardKey: 'f',
      button: 'Y',
    },
    {
      keyboardKey: 'Control',
      button: 'Zl',
    },
    {
      keyboardKey: 'Alt',
      button: 'A',
    },
    {
      keyboardKey: ' ',
      button: 'B',
    },
    {
      keyboardKey: '.',
      button: 'LStick',
    },
    {
      keyboardKey: 'p',
      button: 'Capture',
    },
  ],
  stick: [
    {
      keyboardKey: 'w',
      stick: 'left',
      direction: 'up',
    },
    {
      keyboardKey: 'a',
      stick: 'left',
      direction: 'left',
    },
    {
      keyboardKey: 's',
      stick: 'left',
      direction: 'down',
    },
    {
      keyboardKey: 'd',
      stick: 'left',
      direction: 'right',
    },
    {
      keyboardKey: 'ArrowUp',
      stick: 'right',
      direction: 'up',
    },
    {
      keyboardKey: 'ArrowLeft',
      stick: 'right',
      direction: 'left',
    },
    {
      keyboardKey: 'ArrowDown',
      stick: 'right',
      direction: 'down',
    },
    {
      keyboardKey: 'ArrowRight',
      stick: 'right',
      direction: 'right',
    },
  ],
} satisfies ControllerConfig;

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
  enterControlMode: () => Promise<void>;
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
    } catch (err) {
      setError(err as Error);
    } finally {
      setPending(false);
    }
  }, []);
  const enterControlMode = useCallback(async () => {
    setInControlMode(true);
    await rpcRunControlStream();
    const controllerManager = new ControllerEventManager(CTRL_CONFIG);
    controllerManager.onUpdate((update) => {
      emit('control:input', update);
    });
    await controllerManager.init();
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
    enterControlMode,
  };
}
