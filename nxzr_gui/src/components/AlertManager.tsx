import React, { useCallback, useState } from 'react';
import { Alert, IconName, Intent } from '@blueprintjs/core';
import { createContext, useContext } from 'react';

export interface UseAlertManager {
  open: (options: AlertManagerOpenOptions) => void;
  close: () => void;
}

export interface AlertManagerState {
  isOpen: boolean;
  message?: React.ReactNode;
  intent?: Intent;
  icon?: IconName;
  onConfirm?: () => void;
  onCancel?: () => void;
}

interface AlertManagerOpenOptions {
  message: React.ReactNode;
  intent?: Intent;
  icon?: IconName;
  onConfirm?: () => void;
  onCancel?: () => void;
}

export interface AlertManagerActions {
  state: AlertManagerState;
  open: (options: AlertManagerOpenOptions) => void;
  close: () => void;
}

export const AlertManagerContext = createContext<AlertManagerActions | undefined>(undefined);

const DEFAULT_STATE: AlertManagerState = {
  isOpen: false,
  message: undefined,
  intent: undefined,
  icon: undefined,
  onConfirm: undefined,
  onCancel: undefined,
};

export function AlertManagerProvider(props: React.PropsWithChildren<{}>): React.ReactElement {
  const { children } = props;
  const [state, setState] = useState<AlertManagerState>(DEFAULT_STATE);
  const handleOpen = useCallback((options: AlertManagerOpenOptions) => {
    setState({ ...options, isOpen: true });
  }, []);
  const handleClose = useCallback(() => setState(DEFAULT_STATE), []);
  const handleConfirm = useCallback(() => {
    state.onConfirm?.();
    handleClose();
  }, [state.onConfirm]);
  const value = {
    state,
    open: handleOpen,
    close: handleClose,
  };
  return (
    <AlertManagerContext.Provider value={value}>
      {children}
      <Alert
        className="bp5-dark"
        isOpen={state.isOpen}
        intent={state.intent}
        icon={state.icon}
        onConfirm={handleConfirm}
        onCancel={handleClose}
        onClose={handleClose}
      >
        {state.message}
      </Alert>
    </AlertManagerContext.Provider>
  );
}

export function useAlertManager(): UseAlertManager {
  const context = useContext(AlertManagerContext);
  if (context === undefined) {
    throw new Error('`useAlertManager` must be used within a `AlertManagerProvider`');
  }
  return {
    open: context.open,
    close: context.close,
  };
}
