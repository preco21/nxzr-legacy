import React from 'react';
import classNames from 'classnames';

export interface MainContainerProps {
  className?: string;
  children?: React.ReactNode;
}

export function MainContainer(props: MainContainerProps): React.ReactElement {
  const { className, children } = props;
  return (
    <main className={classNames('container', 'bp4-dark', className)}>
      {children}
    </main>
  );
}
