import React from 'react';
import classNames from 'classnames';
import { styled } from 'styled-components';

export interface MainContainerProps {
  className?: string;
  children?: React.ReactNode;
}

export function MainContainer(props: MainContainerProps): React.ReactElement {
  const { className, children } = props;
  return (
    <Main className={classNames('container', 'bp5-dark', className)}>
      {children}
    </Main>
  );
}

const Main = styled.main`
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
`;
