import React from 'react';
import styled from 'styled-components';
import { Button, ControlGroup, HTMLSelect } from '@blueprintjs/core';
import { AdapterInfo } from '../../common/commands';

export interface AdapterSelectProps {
  className?: string;
  adapters: AdapterInfo[];
  value?: string;
  disabled?: boolean;
  onSelect?: (id: string) => void;
  onRefresh?: () => void;
}

export function AdapterSelect(props: AdapterSelectProps): React.ReactElement {
  const { className, adapters, value, disabled, onSelect, onRefresh } = props;
  return (
    <Container className={className}>
      <ControlGroup>
        <HTMLSelect
          options={[
            {
              label: 'Select an adapter...',
              value: '',
              disabled: true,
            },
            ...adapters.map((adapter) => ({
              label: adapter.name,
              value: adapter.id,
            })),
          ]}
          value={value ?? ''}
          iconName="caret-down"
          disabled={disabled}
          placeholder="Select an adapter..."
          onChange={(e) => onSelect?.(e.target.value)}
        />
        <Button icon="refresh" disabled={disabled} onClick={onRefresh} />
      </ControlGroup>
    </Container>
  );
}

const Container = styled.div`
`;
