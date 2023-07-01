import React, { useCallback, useEffect, useState } from 'react';
import { Button, Tag } from '@blueprintjs/core';
import { launchAgentDaemon, runAgentCheck } from '../common/commands';
import { MainContainer } from '../components/MainContainer';
import { TitleBar } from '../components/TitleBar';
import { Header } from '../components/Header';
import { useAlertManager } from '../components/AlertManager';
import { Setup } from '../features/setup/Setup';
import { StepDisplay, useSetupGuard } from '../features/setup/useSetupGuard';
import { useAdapterManager } from '../features/operation/useAdapterManager';
import { AdapterSelectModal } from '../features/operation/AdapterSelectModal';
import { WslStatus, useWslStatus } from '../features/operation/useWslStatus';
import { useWslAnchor } from '../features/operation/useWslAnchor';

const FAILURE_STATUS: StepDisplay['status'][] = ['checkFailed', 'installFailed'];

let didInit = false;

function AppPage(): React.ReactElement {
  const alertManager = useAlertManager();

  // Wsl
  const wslAnchor = useWslAnchor({
    onFail: useCallback((err: Error) => {
      alertManager.open({
        message: `Failed to launch WSL anchor shell. Please restart the application. (detail: ${err.message})`,
        intent: 'danger',
        icon: 'error',
      });
    }, [alertManager]),
  });

  // Adapter
  const adapterManager = useAdapterManager({
    onAttached: useCallback(async () => {
      //
    }, []),
  });
  const [adapterModalOpen, setAdapterModalOpen] = useState<boolean>(false);

  // Setup
  const setupGuard = useSetupGuard({
    onCheckComplete: useCallback(async () => {
      await wslAnchor.launchAnchorInstance();
      await adapterManager.refreshAdapterList();
    }, []),
    onRebootRequest: useCallback(() => {
      alertManager.open({
        message: 'In order to complete the setup, a reboot is required. Please close the application and restart your computer.',
        intent: 'warning',
        icon: 'warning-sign',
      });
    }, [alertManager]),
  });
  const firstSetupError = setupGuard.steps.find((step) => FAILURE_STATUS.includes(step.status));

  useWslStatus({
    onUpdate: useCallback((status: WslStatus) => {
      if (!status.isReady) {
        adapterManager.reset();
        // FIXME: reset all
        alertManager.open({
          message: 'Has lost connection to WSL. Please restart the application.',
          intent: 'danger',
          icon: 'error',
        });
      }
    }, [alertManager]),
  });

  useEffect(() => {
    if (!didInit) {
      didInit = true;
      // Run a program check at initial render.
      setupGuard.performCheck();
    }
  }, [setupGuard]);

  return (
    <MainContainer>
      <TitleBar />
      <Header
        disabled={!setupGuard.ready}
        adapterInfo={adapterManager.selectedAdapter}
        adapterPending={adapterManager.pending}
        onAdapterDisplayClick={() => setAdapterModalOpen(true)}
      />
      {!setupGuard.ready && (
        <Setup
          steps={setupGuard.steps}
          loading={setupGuard.pending}
          ready={setupGuard.ready}
          error={firstSetupError?.error?.message}
          onInstall={() => setupGuard.performInstall()}
        />
      )}
      <Tag>Wsl status: {wslAnchor.pending ? 'Loading...' : 'Ready'}</Tag>
      {setupGuard.ready && (
        <Button onClick={async () => {
          await runAgentCheck();
          await launchAgentDaemon();
        }}
        >
          Connect to Agent
        </Button>
      )}
      <AdapterSelectModal
        isOpen={adapterModalOpen}
        adapterManager={adapterManager}
        onClose={() => setAdapterModalOpen(false)}
      />
    </MainContainer>
  );
}

export default AppPage;
