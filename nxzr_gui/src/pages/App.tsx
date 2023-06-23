import React, { useEffect, useState } from 'react';
import { MainContainer } from '../components/MainContainer';
import { css } from 'styled-components';
import { TitleBar } from '../components/TitleBar';
import { Header } from '../components/Header';
import { Setup } from '../features/setup/Setup';
import { useSetupGuard } from '../features/setup/useSetupGuard';

function AppPage(): React.ReactElement {
  const setupGuard = useSetupGuard();
  console.log(setupGuard);
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
        />
      )}
      {setupGuard.ready && (
        <div>hooray!</div>
      )}
    </MainContainer>
  );
}

export default AppPage;
