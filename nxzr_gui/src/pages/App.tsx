import React, { useCallback, useEffect, useState } from 'react';
import { css } from 'styled-components';
import { MainContainer } from '../components/MainContainer';
import { TitleBar } from '../components/TitleBar';
import { Header } from '../components/Header';
import { Setup } from '../features/setup/Setup';
import { StepDisplay, useSetupGuard } from '../features/setup/useSetupGuard';
import { RebootAlert } from '../features/setup/RebootAlert';
import { useAdapterManager } from '../features/operation/useAdapterManager';

const FAILURE_STATUS: StepDisplay['status'][] = ['checkFailed', 'installFailed'];

function AppPage(): React.ReactElement {
  const [rebootRequested, setRebootRequested] = useState(false);
  const setupGuard = useSetupGuard({
    onRebootRequest: useCallback(() => {
      setRebootRequested(true);
    }, []),
  });
  const firstSetupError = setupGuard.steps.find((step) => FAILURE_STATUS.includes(step.status));
  const adapterManager = useAdapterManager({
    enabled: setupGuard.ready,
  });
  useEffect(() => {
    // Run a program check at initial render.
    setupGuard.performCheck();
  }, []);
  console.log(adapterManager);
  return (
    <MainContainer>
      <TitleBar />
      <Header disabled={!setupGuard.ready} />
      {!setupGuard.ready && (
        <Setup
          steps={setupGuard.steps}
          loading={setupGuard.pending}
          ready={setupGuard.ready}
          error={firstSetupError?.error?.message}
          outputSink={[]}
          onInstall={() => setupGuard.performInstall()}
        />
      )}
      {setupGuard.ready && (
        <div>hooray!</div>
      )}
      <RebootAlert isOpen={rebootRequested} onConfirm={() => setRebootRequested(false)} />
    </MainContainer>
  );
}

export default AppPage;
