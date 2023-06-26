import React from 'react';
import styled from 'styled-components';
import { Button, Colors, Divider } from '@blueprintjs/core';
import { AdapterInfo, openLogWindow } from '../common/commands';
import { ProfileSelector } from './ProfileSelector';

export interface HeaderProps {
  className?: string;
  disabled?: boolean;
  adapterInfo?: AdapterInfo;
  adapterPending?: boolean;
  onAdapterDisplayClick?: () => void;
}

export function Header(props: HeaderProps): React.ReactElement {
  const { className, disabled, adapterInfo, adapterPending, onAdapterDisplayClick } = props;
  return (
    <Container className={className}>
      <ProfileSelector disabled={disabled} />
      <RightActions>
        <RightActionsInner>
          <AdapterDisplayButton
            icon="antenna"
            disabled={disabled}
            loading={adapterInfo == null && adapterPending}
            small
            onClick={onAdapterDisplayClick}
          >
            Adapter: <i>{adapterInfo?.name ?? 'N/A'}</i>
          </AdapterDisplayButton>
          <Divider />
          <Button
            icon="console"
            minimal
            onClick={() => openLogWindow()}
          />
          <Button icon="cog" disabled={disabled} minimal />
        </RightActionsInner>
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

const RightActionsInner = styled.div`
  display: flex;
`;

const AdapterDisplayButton = styled(Button)`
  align-self: center;
`;
