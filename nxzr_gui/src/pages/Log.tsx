import React, { useEffect, useState } from 'react';
import { MainContainer } from '../components/MainContainer';
import { logger } from '../utils/logger';

function LogPage(): React.ReactElement {
  const [logs, setLogs] = useState<string[]>([]);

  useEffect(() => {
    const handle = setInterval(() => {
      console.log(123);
      logger.info('foobar');
    }, 1000);
    return () => clearInterval(handle);
  }, []);

  useEffect(() => {
    let handle: () => void;
    (async () => {
      await logger.init();
      const initialLogs = logger.logs;
      setLogs(initialLogs);
      handle = logger.onLog((log) => setLogs((prev) => [...prev, log]));
    })();
    return () => {
      if (handle != null) {
        handle();
      }
      logger.dispose();
    };
  }, []);
  return (
    <MainContainer>
      {logs.join('\n')}
    </MainContainer>
  );
}

export default LogPage;
