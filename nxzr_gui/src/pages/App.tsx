import React, { useCallback, useEffect, useState } from 'react';
import { css } from 'styled-components';
import { MainContainer } from '../components/MainContainer';
import { TitleBar } from '../components/TitleBar';
import { Header } from '../components/Header';
import { Setup } from '../features/setup/Setup';
import { useSetupGuard } from '../features/setup/useSetupGuard';
import { RebootAlert } from '../features/setup/RebootAlert';

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
      <RebootAlert isOpen={rebootRequested} onConfirm={() => setRebootRequested(false)} />
    </MainContainer>
  );
}

export default AppPage;
