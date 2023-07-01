import React, { useCallback, useEffect, useState } from 'react';
import { launchAgentDaemon, launchWslAnchorInstance, runAgentCheck } from '../common/commands';
import { MainContainer } from '../components/MainContainer';
import { TitleBar } from '../components/TitleBar';
import { Header } from '../components/Header';
import { Setup } from '../features/setup/Setup';
import { StepDisplay, useSetupGuard } from '../features/setup/useSetupGuard';
import { useAdapterManager } from '../features/operation/useAdapterManager';
import { AdapterSelectModal } from '../features/operation/AdapterSelectModal';
import { Button } from '@blueprintjs/core';
import { useAlertManager } from '../components/AlertManager';

const FAILURE_STATUS: StepDisplay['status'][] = ['checkFailed', 'installFailed'];

let didInit = false;

function AppPage(): React.ReactElement {
  const alertManager = useAlertManager();

  // Adapter
  const adapterManager = useAdapterManager({ });
  const [adapterModalOpen, setAdapterModalOpen] = useState<boolean>(false);

  // Setup
  const setupGuard = useSetupGuard({
    onCheckComplete: useCallback(async () => {
      await launchWslAnchorInstance();
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
