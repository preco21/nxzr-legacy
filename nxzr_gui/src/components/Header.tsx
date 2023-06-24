import React from 'react';
import styled from 'styled-components';
import { Button, Colors } from '@blueprintjs/core';
import { openLogWindow } from '../common/commands';
import { ProfileSelector } from './ProfileSelector';

export interface HeaderProps {
  className?: string;
  disabled?: boolean;
}

export function Header(props: HeaderProps): React.ReactElement {
  const { className, disabled } = props;
  return (
    <Container className={className}>
      <ProfileSelector disabled={disabled} />
      <RightActions>
        <Button
          icon="console"
          minimal
          onClick={() => openLogWindow()}
        />
        <Button icon="cog" disabled={disabled} minimal />
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
