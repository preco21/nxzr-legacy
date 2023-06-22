import { Button, NonIdealState, Spinner } from '@blueprintjs/core';
import React from 'react';
import styled from 'styled-components';
// import { useSetupGuard } from './useSetupGuard';

export interface SetupProps {
  className?: string;
  loading: boolean;
  title?: string;
  description?: string;
}

export function Setup(props: SetupProps): React.ReactElement {
  const { className, loading, title, description } = props;
  return (
    <Container className={className}>
      <NonIdealState
        icon={loading ? <Spinner size={50} /> : 'issue'}
        title={title ?? 'Setup Required'}
        description={description}
        action={<Button>foobar</Button>}
      />
    </Container>
  );
}

const Container = styled.div`
  height: 100%;
`;
