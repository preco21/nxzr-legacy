import React from 'react';
import styled from 'styled-components';
import { Button, Colors } from '@blueprintjs/core';
import { ProfileSelector } from './ProfileSelector';

export interface HeaderProps {
  className?: string;
  children?: React.ReactNode;
}

export function Header(props: HeaderProps): React.ReactElement {
  const { className, children } = props;
  return (
    <Container className={className}>
      <ProfileSelector />
      <RightActions>
        <Button icon="cog" minimal />
      </RightActions>
    </Container>
  );
}

const Container = styled.div`
  display: flex;
  justify-content: space-between;
  padding: 12px;
  background-color: ${Colors.DARK_GRAY2};
`;

const RightActions = styled.div`
  display: flex;
  align-items: flex-start;
`;
