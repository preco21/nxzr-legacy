import React from 'react';
import styled from 'styled-components';
import { Button, Divider, HTMLSelect, InputGroup } from '@blueprintjs/core';

export interface ProfileSelectorProps {
  className?: string;
  disabled?: boolean;
}

export function ProfileSelector(props: ProfileSelectorProps): React.ReactElement {
  const { className, disabled } = props;
  return (
    <Container className={className}>
      <Row>
        <HTMLSelect options={[{ value: 'foobar', label: '프로필1' }]} disabled={disabled} />
        <Divider />
        <ActionButtons>
          <Button icon="plus" disabled={disabled} minimal small />
          <Button icon="annotation" disabled={disabled} minimal small />
          <Button icon="trash" disabled={disabled} minimal small />
          <Button icon="duplicate" disabled={disabled} minimal small />
        </ActionButtons>
      </Row>
      <Row>
        <InputGroup placeholder="Ctrl + Shift + 1" disabled={disabled} />
      </Row>
    </Container>
  );
}

const Container = styled.div`
`;

const Row = styled.div`
  display: flex;
  &:not(:last-of-type) {
    margin-bottom: 8px;
  }
`;

const ActionButtons = styled.div`
  display: flex;
  align-items: center;
`;
