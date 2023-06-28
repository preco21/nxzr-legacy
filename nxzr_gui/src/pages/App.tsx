import React, { useCallback, useEffect, useState } from 'react';
import { css } from 'styled-components';
import { launchWslInstance, runWslAgentCheck } from '../common/commands';
import { MainContainer } from '../components/MainContainer';
import { TitleBar } from '../components/TitleBar';
import { Header } from '../components/Header';
import { Setup } from '../features/setup/Setup';
import { StepDisplay, useSetupGuard } from '../features/setup/useSetupGuard';
import { RebootAlert } from '../features/setup/RebootAlert';
import { useAdapterManager } from '../features/operation/useAdapterManager';
import { AdapterSelectModal } from '../features/operation/AdapterSelectModal';

const FAILURE_STATUS: StepDisplay['status'][] = ['checkFailed', 'installFailed'];

function AppPage(): React.ReactElement {
  const [rebootRequested, setRebootRequested] = useState(false);
  const adapterManager = useAdapterManager({
  });
  const setupGuard = useSetupGuard({
    onCheckComplete: useCallback(async () => {
      await launchWslInstance();
      await adapterManager.refreshAdapterList();
      await runWslAgentCheck();
    }, []),
    onRebootRequest: useCallback(() => {
      setRebootRequested(true);
    }, []),
  });
  const firstSetupError = setupGuard.steps.find((step) => FAILURE_STATUS.includes(step.status));

  const [adapterModalOpen, setAdapterModalOpen] = useState<boolean>(false);
  useEffect(() => {
    // Run a program check at initial render.
    setupGuard.performCheck();
  }, []);
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
        <div>hooray!</div>
      )}
      <RebootAlert isOpen={rebootRequested} onConfirm={() => setRebootRequested(false)} />
      <AdapterSelectModal
        isOpen={adapterModalOpen}
        adapterManager={adapterManager}
        onClose={() => setAdapterModalOpen(false)}
      />
    </MainContainer>
  );
}

export default AppPage;
