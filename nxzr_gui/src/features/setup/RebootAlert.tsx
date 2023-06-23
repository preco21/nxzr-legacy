import React from 'react';
import styled from 'styled-components';
import { Alert } from '@blueprintjs/core';

export interface RebootAlertProps {
  className?: string;
  isOpen: boolean;
  onConfirm: () => void;
}

export function RebootAlert(props: RebootAlertProps): React.ReactElement {
  const { className, isOpen, onConfirm } = props;
  return (
    <Container className={className}>
      <Alert
        className="bp5-dark"
        isOpen={isOpen}
        intent="warning"
        icon="warning-sign"
        onConfirm={onConfirm}
      >
        In order to complete the setup, a reboot is required.
        Please close the application and restart your computer.
      </Alert>
    </Container>
  );
}

const Container = styled.div`
`;
