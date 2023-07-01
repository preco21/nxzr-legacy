import { Button, Callout, Icon, NonIdealState, Spinner } from '@blueprintjs/core';
import React from 'react';
import styled from 'styled-components';
import { StepDisplay } from './useSetupGuard';
import { StepStatus } from './StepStatus';

export interface SetupProps {
  className?: string;
  steps: StepDisplay[];
  loading: boolean;
  ready: boolean;
  installRequired: boolean;
  error?: React.ReactNode;
  onInstall: () => void;
}

export function Setup(props: SetupProps): React.ReactElement {
  const {
    className,
    steps,
    loading,
    ready,
    installRequired,
    error,
    onInstall,
  } = props;
  const [title, description] = buildStatusText(loading, ready);
  return (
    <Container className={className}>
      <NonIdealState
        icon={(
          loading
            ? <Spinner size={50} />
            : <StatusIcon icon="warning-sign" intent="warning" size={48} />
        )}
        title={title}
        description={(
          <>
            <span>{description}</span>
            <StatusDisplay title="Check status">
              {steps.map((step, index) => (
                <StepStatus key={index} stepDisplay={step} />
              ))}
            </StatusDisplay>
          </>
        )}
        action={(
          <>
            {error != null && (
              <Callout intent="danger">
                {error}
              </Callout>
            )}
            {!loading && installRequired && (
              <Button
                icon="archive"
                intent="success"
                large
                onClick={onInstall}
              >
                Install
              </Button>
            )}
          </>
        )}
      />
    </Container>
  );
}

function buildStatusText(loading: boolean, ready: boolean): [title: string, description: string] {
  if (loading) {
    return ['Validating program requirements...', 'This step may take a while.'];
  }
  if (ready) {
    return ['Program ready', 'Check completed, program is ready to use.'];
  }
  return ['Setup required', 'Program requirements not met, initial setup required.'];
}

const Container = styled.div`
  height: 100%;
`;

const StatusDisplay = styled.div`
  margin: 12px;
`;

const StatusIcon = styled(Icon)`
  &&& > svg {
    fill-opacity: 100%;
  }
`;
