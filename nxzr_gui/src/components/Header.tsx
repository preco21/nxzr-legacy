import { invoke } from '@tauri-apps/api/tauri';
import React from 'react';
import styled from 'styled-components';
import { Button, Colors } from '@blueprintjs/core';
import { ProfileSelector } from './ProfileSelector';

export interface HeaderProps {
  className?: string;
}

export function Header(props: HeaderProps): React.ReactElement {
  const { className } = props;
  return (
    <Container className={className}>
      <ProfileSelector />
      <RightActions>
        <Button
          icon="console"
          minimal
          onClick={async () => {
            await invoke('open_log_window');
          }}
        />
        <Button icon="cog" minimal />
      </RightActions>
    </Container>
  );
}

const Container = styled.div`
  display: flex;
  justify-content: space-between;
  padding: 12px 8px;
  background-color: ${Colors.BLACK};
`;

const RightActions = styled.div`
  display: flex;
  align-items: flex-start;
`;
