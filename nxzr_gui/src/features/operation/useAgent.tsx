// 1. agent check 실행 -> 실패시 프로그램 다시 키도록 가이드 (fatal error)
// FIXME: 어뎁터 변경시(useAdapterManager) nxzr server 종료 처리하고 다시 시작 필요 (3번과 동일) - 근데 변경 커맨드 실행 전에 해야 하는게 맞는 것 같은데...
// 2. agent check 성공시 agent daemon 실행
// 3. agent daemon 실행 중 터질 경우 이벤트 받아서 에러만 alert (warn error)로 표시하고, 다시 실행, 연결은 다시 안 함 -> 그래도 터지면 프로그램 다시 키도록 가이드

import { useCallback, useState } from 'react';
import { runAgentCheck } from '../../common/commands';

export interface UseAgent {
  pending: boolean;
  isReady: boolean;
  error?: Error;
  launchAgentDaemon: () => Promise<void>;
  // shutdownAgentDaemon: () => Promise<void>;
}

export interface UseAgentOptions {
  onFailure?: (error: Error) => void;
}

export function useAgent(options?: UseAgentOptions): UseAgent {
  const [isReady, setIsReady] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<Error | undefined>(undefined);
  const launchAgentDaemon = useCallback(async () => {
    try {
      setPending(true);
      await runAgentCheck();
      await launchAgentDaemon();
      setIsReady(true);
    } catch (err) {
      setError(err as Error);
      options?.onFailure?.(err as Error);
    } finally {
      setPending(false);
    }
  }, []);
  // FIXME: handle events: agent:status_change
  return {
    pending,
    isReady,
    error,
    launchAgentDaemon,
  };
}
