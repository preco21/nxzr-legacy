import React from 'react';
import styled from 'styled-components';
import { IconName, Intent, Tag } from '@blueprintjs/core';
import { StepDisplay } from './useSetupGuard';

const STEP_STATUS_VISUAL_MAP = {
  none: {
    intent: 'none',
    icon: 'ban-circle',
    text: '-',
  },
  wait: {
    intent: 'primary',
    icon: 'time',
    text: 'Waiting...',
  },
  check: {
    intent: 'warning',
    icon: 'history',
    text: 'Checking...',
  },
  checkFailed: {
    intent: 'danger',
    icon: 'error',
    text: 'Check failed',
  },
  install: {
    intent: 'warning',
    icon: 'download',
    text: 'Installing...',
  },
  installFailed: {
    intent: 'danger',
    icon: 'cross-circle',
    text: 'Install failed',
  },
  ready: {
    intent: 'success',
    icon: 'tick-circle',
    text: 'Ready',
  },
  rebootRequired: {
    intent: 'warning',
    icon: 'refresh',
    text: 'Reboot required',
  },
  skipped: {
    intent: 'none',
    icon: 'disable',
    text: 'Skipped',
  },
} satisfies Record<StepDisplay['status'], {
  intent: Intent;
  icon: IconName;
  text: string;
}>;

export interface StepStatusProps {
  stepDisplay: StepDisplay;
}

export function StepStatus(props: StepStatusProps): React.ReactElement {
  const { stepDisplay } = props;
  const desiredStatus = STEP_STATUS_VISUAL_MAP[stepDisplay.status];
  return (
    <Wrapper>
      <StatusTag
        intent={desiredStatus.intent}
        icon={desiredStatus.icon}
        minimal
        large
        fill
      >
        <InnerTagContainer>
          {desiredStatus.text}
        </InnerTagContainer>
        {' '}
        {stepDisplay.name}
      </StatusTag>
    </Wrapper>
  );
}

const Wrapper = styled.div`
  margin: 6px;
`;

const StatusTag = styled(Tag)`
  text-align: left;
`;

const InnerTagContainer = styled.div`
  display: inline-flex;
  flex: 1;
  min-width: 105px;
  margin: 0 2px;
`;
