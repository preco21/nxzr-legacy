import { UnlistenFn, listen } from '@tauri-apps/api/event';
import { appWindow } from '@tauri-apps/api/window';
import React, { useEffect, useState } from 'react';
import styled from 'styled-components';
import { Button, Colors } from '@blueprintjs/core';

export interface TitleBarProps {
  className?: string;
}

export function TitleBar(props: TitleBarProps): React.ReactElement {
  const { className } = props;
  const [isMaximized, setMaximized] = useState(false);

  useEffect(() => {
    let unlisten: UnlistenFn;
    (async () => {
      unlisten = await listen('tauri://resize', async () => {
        const maximized = await appWindow.isMaximized();
        setMaximized(maximized);
      });
    })();
    return () => unlisten?.();
  }, []);

  return (
    <Container className={className} data-tauri-drag-region>
      <TitleArea>
        <Title>NXZR</Title>
        <VersionPlace>v{APP_VERSION}</VersionPlace>
      </TitleArea>
      <WindowActions>
        <Button icon="minus" minimal onClick={() => appWindow.minimize()} />
        <Button
          icon={isMaximized ? 'duplicate' : 'small-square'}
          minimal
          onClick={async () => {
            const maximized = await appWindow.isMaximized();
            if (maximized) {
              await appWindow.unmaximize();
            } else {
              await appWindow.maximize();
            }
          }}
        />
        <Button icon="cross" minimal onClick={() => appWindow.close()} />
      </WindowActions>
    </Container>
  );
}

const Container = styled.header`
  user-select: none;
  display: grid;
  grid-template: 1fr / 12fr 4fr;
  padding: 4px 8px;
  background-color: ${Colors.BLACK};
`;

const TitleArea = styled.div`
  pointer-events: none;
  display: flex;
  align-items: center;
`;

const Title = styled.h3`
  margin: 0;
`;

const VersionPlace = styled.h5`
  margin: 3px 8px 0;
`;

const WindowActions = styled.div`
  pointer-events: none;
  display: flex;
  justify-content: flex-end;
  > * {
    pointer-events: all;
  }
`;
