import React from 'react';
import styled from 'styled-components';
import { Button, Divider, HTMLSelect, InputGroup } from '@blueprintjs/core';

export interface ProfileSelectorProps {
  className?: string;
  children?: React.ReactNode;
}

export function ProfileSelector(props: ProfileSelectorProps): React.ReactElement {
  const { className, children } = props;
  return (
    <Container className={className}>
      <Row>
        <HTMLSelect options={[{ value: 'foobar', label: '프로필1' }]} />
        <Divider />
        <ActionButtons>
          <Button icon="plus" minimal small />
          <Button icon="annotation" minimal small />
          <Button icon="trash" minimal small />
          <Button icon="duplicate" minimal small />
        </ActionButtons>
      </Row>
      <Row>
        <InputGroup placeholder="Ctrl + Shift + 1" />
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
