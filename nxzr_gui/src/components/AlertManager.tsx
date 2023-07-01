import React, { useCallback, useMemo, useState } from 'react';
import { Alert, IconName, Intent } from '@blueprintjs/core';
import { createContext, useContext } from 'react';
import { generateId } from '../utils/general';

export type UseAlertManager = AlertManagerActions;

export interface AlertManagerState {
  id: number;
  isOpen: boolean;
  message?: React.ReactNode;
  intent?: Intent;
  icon?: IconName;
  onConfirm?: () => void;
  onCancel?: () => void;
}

export interface AlertManagerActions {
  open: (options: AlertManagerOpenOptions) => number;
  close: (id: number) => void;
}

export interface AlertManagerOpenOptions {
  message: React.ReactNode;
  intent?: Intent;
  icon?: IconName;
  onConfirm?: () => void;
  onCancel?: () => void;
}

export const AlertManagerContext = createContext<AlertManagerActions | undefined>(undefined);

const PARTIAL_DEFAULT: Partial<AlertManagerState> = {
  message: undefined,
  intent: undefined,
  icon: undefined,
  onConfirm: undefined,
  onCancel: undefined,
};

export function AlertManagerProvider(props: React.PropsWithChildren<{}>): React.ReactElement {
  const { children } = props;
  const [queue, setQueue] = useState<AlertManagerState[]>([]);
  const handleOpen = useCallback((options: AlertManagerOpenOptions) => {
    const id = generateId();
    setQueue((prev) => [...prev, { ...PARTIAL_DEFAULT, ...options, id, isOpen: true }]);
    return id;
  }, []);
  const handleClose = useCallback((id: number) => {
    setQueue((prev) => prev.filter((item) => item.id === id));
  }, []);

  const value = useMemo(() => ({
    open: handleOpen,
    close: handleClose,
  } satisfies AlertManagerActions), [
    handleOpen,
    handleClose,
  ]);
  return (
    <AlertManagerContext.Provider value={value}>
      {children}
      {queue.map((item) => (
        <Alert
          key={item.id}
          className="bp5-dark"
          isOpen={item.isOpen}
          intent={item.intent}
          icon={item.icon}
          onConfirm={() => {
            item.onConfirm?.();
            handleClose(item.id);
          }}
          onCancel={() => handleClose(item.id)}
          onClose={() => handleClose(item.id)}
        >
          {item.message}
        </Alert>
      ))}
    </AlertManagerContext.Provider>
  );
}

export function useAlertManager(): UseAlertManager {
  const value = useContext(AlertManagerContext);
  if (value === undefined) {
    throw new Error('`useAlertManager` must be used within a `AlertManagerProvider`');
  }
  return value;
}
