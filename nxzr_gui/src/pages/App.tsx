import React, { useCallback, useEffect, useState } from 'react';
import { styled } from 'styled-components';
import { Button, Tag, TextArea } from '@blueprintjs/core';
import { preventCursorEscape } from '../utils/control';
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
import { useAgent } from '../features/operation/useAgent';

const FAILURE_STATUS: StepDisplay['status'][] = ['checkFailed', 'installFailed'];

let didInit = false;

function AppPage(): React.ReactElement {
  const alertManager = useAlertManager();

  // Wsl
  const wslAnchor = useWslAnchor({
    onFailure: useCallback((err: Error) => {
      alertManager.open({
        message: `Failed to launch WSL anchor shell. Please restart the application. (detail: ${err.message})`,
        intent: 'danger',
        icon: 'error',
      });
    }, [alertManager]),
  });

  // Agent
  const agent = useAgent({
    onLaunchFailed: useCallback((err: Error) => {
      alertManager.open({
        message: `Failed to launch the agent. Please restart the application. (detail: ${err.message})`,
        intent: 'danger',
        icon: 'error',
      });
    }, [alertManager]),
  });

  // Adapter
  const adapterManager = useAdapterManager({
    onDetached: useCallback(async () => {
      await agent.terminateDaemon();
    }, []),
    onAttached: useCallback(async () => {
      await agent.launchDaemon();
    }, [agent]),
    onAdapterAutoSelected: useCallback(async () => {
      await agent.launchDaemon();
    }, [agent]),
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

  const wslStatusText = (() => {
    if (wslAnchor.pending) {
      return 'Loading...';
    }
    if (!wslAnchor.isReady) {
      return 'Not ready';
    }
    return 'Ready';
  })();

  const agentStatusText = (() => {
    if (agent.pending) {
      return 'Loading...';
    }
    if (!agent.isReady) {
      return 'Not ready';
    }
    return 'Ready';
  })();

  return (
    <MainContainer>
      <TitleBar />
      <Header
        disabled={!setupGuard.isReady || agent.inControlMode}
        adapterInfo={adapterManager.selectedAdapter}
        adapterPending={adapterManager.pending}
        onAdapterDisplayClick={() => setAdapterModalOpen(true)}
      />
      {!setupGuard.isReady && (
        <Setup
          steps={setupGuard.steps}
          loading={setupGuard.pending}
          ready={setupGuard.isReady}
          installRequired={setupGuard.installRequired}
          error={firstSetupError?.error?.message}
          onInstall={() => setupGuard.performInstall()}
        />
      )}
      {setupGuard.isReady && (
        <Body>
          <Tag>Wsl status: {wslStatusText}</Tag>
          <Tag>Agent status: {agentStatusText}</Tag>
          <TextArea
            value={JSON.stringify(agent.deviceStatus)}
            fill
            readOnly
          />
          <Button
            disabled={!agent.isReady || agent.switchConnected}
            loading={agent.pending}
            onClick={() => agent.connectSwitch()}
          >
            Connect Switch
          </Button>
          {agent.inControlMode && <InControlModeOverlay />}
          <Button onClick={async () => {
            // invoke('lock_cursor');
            // document.body.requestPointerLock();
            // navigator.keyboard.lock(['Escape']);
            // document.addEventListener('pointerlockchange', (e) => {
            //   console.log('foobar', document.pointerLockElement);
            //   if (document.pointerLockElement == null) {
            //     e.preventDefault();
            //     setTimeout(() => {
            //       document.body.requestPointerLock();
            //     }, 300);
            //   }
            // });

            let x = 0;
            let y = 0;
            await preventCursorEscape((e) => {
              x += e.x;
              y += e.y;
              console.log(`${x}, ${y}`);
            });
          }}
          >
            Pointer lock
          </Button>
        </Body>
      )}
      <AdapterSelectModal
        isOpen={adapterModalOpen}
        adapterManager={adapterManager}
        onClose={() => setAdapterModalOpen(false)}
      />
    </MainContainer>
  );
}

const Body = styled.section`
  position: relative;
`;

const InControlModeOverlay = styled.div`
  position: absolute;
  top: 0;
  right: 0;
  left: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  cursor: not-allowed;
`;

export default AppPage;
