import React, { useCallback, useEffect, useState } from 'react';
import { css } from 'styled-components';
import { Alert } from '@blueprintjs/core';
import { MainContainer } from '../components/MainContainer';
import { TitleBar } from '../components/TitleBar';
import { Header } from '../components/Header';
import { Setup } from '../features/setup/Setup';
import { useSetupGuard } from '../features/setup/useSetupGuard';

function AppPage(): React.ReactElement {
  const [rebootRequested, setRebootRequested] = useState(false);
  const setupGuard = useSetupGuard({
    onRebootRequest: useCallback(() => {
      setRebootRequested(true);
    }, []),
  });
  useEffect(() => {
    // Run a program check at initial render.
    setupGuard.performCheck();
  }, []);
  return (
    <MainContainer>
      <TitleBar />
      <Header disabled={!setupGuard.ready} />
      {!setupGuard.ready && (
        <Setup
          steps={setupGuard.steps}
          loading={setupGuard.pending}
          ready={setupGuard.ready}
          onInstall={() => setupGuard.performInstall()}
        />
      )}
      {setupGuard.ready && (
        <div>hooray!</div>
      )}
      <Alert
        className="bp5-dark"
        isOpen={rebootRequested}
        intent="warning"
        icon="warning-sign"
        onConfirm={() => setRebootRequested(false)}
      >
        In order to complete the setup, a reboot is required.
        Please close the application and restart your computer.
      </Alert>
    </MainContainer>
  );
}

export default AppPage;
