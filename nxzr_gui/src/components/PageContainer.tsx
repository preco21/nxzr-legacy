import React from 'react';
import classNames from 'classnames';

export interface PageContainerProps {
  className?: string;
  children?: React.ReactNode;
}

export function PageContainer(props: PageContainerProps): React.ReactElement {
  const { className, children } = props;
  return (
    <main className={classNames('container', 'bp5-dark', className)}>
      {children}
    </main>
  );
}
