import React, { useEffect, useRef, useState } from 'react';
import * as logger from '../utils/logger';
import { MainContainer } from '../components/MainContainer';
import { Console } from 'console-feed';
import styled from 'styled-components';
import { Button, Checkbox, Colors, ControlGroup, HTMLSelect, InputGroup, OptionProps } from '@blueprintjs/core';

type ConsoleProps = React.ComponentProps<typeof Console>;
type ConsoleMessage = ConsoleProps['logs'][number];
type ConsoleMethod = ConsoleMessage['method'];

const LOG_LEVEL_TO_METHOD_MAP = {
  ERROR: 'error',
  WARN: 'warn',
  INFO: 'info',
  DEBUG: 'debug',
  TRACE: 'log',
} satisfies Record<logger.LogLevel, ConsoleMethod>;

const LOG_LEVEL_SELECT_OPTIONS = [
  {
    label: 'All',
    value: 'all',
  },
  {
    label: 'Error',
    value: 'error',
  },
  {
    label: 'Warn',
    value: 'warn',
  },
  {
    label: 'Info',
    value: 'info',
  },
  {
    label: 'Debug',
    value: 'debug',
  },
  {
    label: 'Log',
    value: 'log',
  },
] as const;

function LogPage(): React.ReactElement {
  const consoleRef = useRef<HTMLDivElement | null>(null);
  const [logs, setLogs] = useState<ConsoleMessage[]>([]);
  const [logFilterKeyword, setLogFilterKeyword] = useState('');
  const [desiredLogLevel, setDesiredLogLevel] = useState<ConsoleMethod | 'all'>('all');
  const [autoScroll, setAutoScroll] = useState(true);

  const autoScrollRef = useRef<boolean>();
  autoScrollRef.current = autoScroll;
  useEffect(() => {
    // Set initial logs by copying the currently stored logs.
    //
    // Otherwise, the initial logs will be incorrectly set with duplicated entries.
    const initialLogs = logger.logListener.initialLogs.map(convertLogEntryToMessage);
    setLogs(initialLogs);
    const unsubscribe = logger.logListener.onLog((entry) => {
      setLogs((prev) => [...prev, convertLogEntryToMessage(entry)]);
      if (autoScrollRef.current) {
        requestAnimationFrame(() => {
          consoleRef.current?.scrollTo(0, consoleRef.current.scrollHeight);
        });
      }
    });
    return () => {
      unsubscribe();
    };
  }, []);

  return (
    <Container>
      <ConsoleActions>
        <Button icon="trash" minimal onClick={() => setLogs([])} />
        <InputGroup
          type="text"
          placeholder="Filter logs..."
          value={logFilterKeyword}
          leftIcon="filter"
          rightElement={(
            logFilterKeyword.length > 0
              ? <Button icon="cross" minimal onClick={() => setLogFilterKeyword('')} />
              : undefined
          )}
          fill
          onChange={(e) => setLogFilterKeyword((e.target as HTMLInputElement).value)}
        />
        <LogLevelSelect
          placeholder="Log level..."
          options={LOG_LEVEL_SELECT_OPTIONS}
          iconName="caret-down"
          value={desiredLogLevel}
          minimal
          onChange={(e) => setDesiredLogLevel((e.target as HTMLSelectElement).value as ConsoleMethod | 'all')}
        />
        <AutoScrollCheckbox
          label="Auto-scroll"
          checked={autoScroll}
          onChange={(e) => setAutoScroll((e.target as HTMLInputElement).checked)}
        />
      </ConsoleActions>
      <ConsoleContainer ref={consoleRef}>
        <Console
          logs={logs}
          variant="dark"
          filter={desiredLogLevel === 'all' ? undefined : [desiredLogLevel]}
          searchKeywords={logFilterKeyword}
        />
      </ConsoleContainer>
    </Container>
  );
}

const Container = styled(MainContainer)`
  display: flex;
  flex-direction: column;
`;

const LogLevelSelect = styled(HTMLSelect)`
  min-width: 150px;
`;

const ConsoleContainer = styled.section`
  overflow: auto;
`;

const ConsoleActions = styled(ControlGroup)`
  padding: 3px;
  background-color: ${Colors.DARK_GRAY3};
  align-items: center;
`;

const AutoScrollCheckbox = styled(Checkbox)`
  &&& {
    margin: 0 12px;
  }
`;

function convertLogEntryToMessage(entry: logger.LogEntry): ConsoleMessage {
  return {
    id: String(entry.id),
    method: LOG_LEVEL_TO_METHOD_MAP[entry.level],
    timestamp: getTimestampString(entry.timestamp),
    data: [`[${entry.target}]`, entry.fields.message + Math.random()],
  };
}

function getTimestampString(timestamp: string): string {
  const date = new Date(timestamp);
  const h = getNumberStringWithWidth(date.getHours(), 2);
  const min = getNumberStringWithWidth(date.getMinutes(), 2);
  const sec = getNumberStringWithWidth(date.getSeconds(), 2);
  const ms = getNumberStringWithWidth(date.getMilliseconds(), 3);
  return `${h}:${min}:${sec}.${ms}`;
}

function getNumberStringWithWidth(num: number, width: number): string {
  const str = String(num);
  if (width > str.length) {
    return `${'0'.repeat(width - str.length)}${str}`;
  }
  return str.slice(0, width + 1);
}

export default LogPage;
