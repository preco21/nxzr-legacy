import React, { useEffect, useState } from 'react';
import * as logger from '../utils/logger';
import { MainContainer } from '../components/MainContainer';

function LogPage(): React.ReactElement {
  const [logs, setLogs] = useState<string[]>([]);

  useEffect(() => {
    const handle = setInterval(() => {
      logger.info('foobar');
    }, 1000);
    return () => clearInterval(handle);
  }, []);

  useEffect(() => {
    // Set initial logs by copying the currently stored logs.
    setLogs(logger.logListener.initialLogs.slice());
    const unsubscribe = logger.logListener.onLog((log) => setLogs((prev) => [...prev, log]));
    return () => {
      unsubscribe();
    };
  }, []);
  return (
    <MainContainer>
      {logs.join('\n')}
    </MainContainer>
  );
}

export default LogPage;
