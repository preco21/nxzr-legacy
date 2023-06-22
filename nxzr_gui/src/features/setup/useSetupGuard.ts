import { produce } from 'immer';
import { invoke } from '@tauri-apps/api/tauri';
import { useCallback, useEffect, useMemo, useState } from 'react';

interface SetupStep {
  name: string;
  description: string;
  rebootRequired: boolean;
  check: () => Promise<void>;
  install: () => Promise<void>;
}

const SETUP_STEPS: SetupStep[] = [
  {
    name: 'Program Setup',
    description: 'This step will install all the necessary components of the program on your computer.',
    rebootRequired: true,
    check: async () => invoke('check_1_setup_installed'),
    install: async () => { /* noop */ },
  },
  {
    name: 'WSL Global Configuration',
    description: 'This step will ensure that WSL is configured correctly on your computer.',
    rebootRequired: false,
    check: async () => invoke('check_2_wslconfig'),
    install: async () => { /* noop */ },
  },
  {
    name: 'Agent Registration',
    description: 'This step will register the server daemon with the WSL instance.',
    rebootRequired: false,
    check: async () => invoke('check_3_agent_registered'),
    install: async () => { /* noop */ },
  },
];

export interface UseSetupGuardOptions {
  onCheckComplete?: () => void;
  onCheckError?: (error: Error) => void;
  onInstallComplete?: () => void;
  onInstallError?: (error: Error) => void;
  onRebootRequest: () => void;
}

export interface UseSetupGuardState {
  pending: boolean;
  ready: boolean;
  steps: StepDisplay[];
  currentStepIndex?: number;
  outputSink: string[];
}

export interface UseSetupGuard {
  pending: boolean;
  ready: boolean;
  steps: StepDisplay[];
  currentStep?: StepDisplay;
  outputSink: string[];
  performCheck: () => void;
  // performInstall: () => void;
}

export interface StepDisplay {
  name: string;
  description: string;
  rebootRequired: boolean;
  checkStatus: 'none' | 'wait' | 'check' | 'ready' | 'error' | 'aborted';
  installRequested: boolean;
  installStatus?: 'wait' | 'install' | 'ready' | 'error' | 'aborted';
  error?: Error;
}

export function useSetupGuard(options?: UseSetupGuardOptions): UseSetupGuard {
  const [state, setState] = useState<UseSetupGuardState>(() => ({
    pending: false,
    ready: false,
    steps: SETUP_STEPS.map((step) => ({
      name: step.name,
      description: step.description,
      rebootRequired: step.rebootRequired ?? false,
      checkStatus: 'none',
      installRequested: false,
      installStatus: undefined,
      error: undefined,
    })),
    currentStepIndex: undefined,
    outputSink: [],
  }));
  const performCheck = useCallback(async () => {
    if (state.pending) {
      return;
    }
    setState((prevState) => produce(prevState, (draft) => {
      draft.pending = true;
      for (const step of draft.steps) {
        step.checkStatus = 'wait';
        step.error = undefined;
      }
      draft.currentStepIndex = undefined;
      draft.outputSink = [];
    }));
    let aborted = false;
    for (const [index, step] of SETUP_STEPS.entries()) {
      try {
        if (aborted) {
          setState((prevState) => produce(prevState, (draft) => {
            draft.steps[index]!.checkStatus = 'aborted';
          }));
          continue;
        }
        setState((prevState) => produce(prevState, (draft) => {
          draft.currentStepIndex = index;
          draft.steps[index]!.checkStatus = 'check';
        }));
        await step.check();
        setState((prevState) => produce(prevState, (draft) => {
          draft.steps[index]!.checkStatus = 'ready';
        }));
      } catch (err) {
        aborted = true;
        setState((prevState) => produce(prevState, (draft) => {
          draft.steps[index]!.checkStatus = 'error';
          draft.steps[index]!.error = err as Error;
        }));
      }
    }
    setState((prevState) => ({ ...prevState, pending: false, ready: !aborted }));
  }, [state.pending]);
  // const process = useCallback(async () => {
  //   if (state.pending) {
  //     return;
  //   }
  //   setState((prevState) => produce(prevState, (draft) => {
  //     draft.pending = true;
  //     for (const step of draft.steps) {
  //       step.checkStatus = 'wait';
  //       step.error = undefined;
  //     }
  //     draft.currentStepIndex = undefined;
  //     draft.outputSink = [];
  //   }));
  //   let aborted = false;
  //   for (const [index, step] of SETUP_STEPS.entries()) {
  //     try {
  //       if (aborted) {
  //         setState((prevState) => produce(prevState, (draft) => {
  //           draft.steps[index]!.checkStatus = 'aborted';
  //         }));
  //         continue;
  //       }
  //       setState((prevState) => produce(prevState, (draft) => {
  //         draft.currentStepIndex = index;
  //         draft.steps[index]!.checkStatus = 'check';
  //       }));
  //       await step.check();
  //       setState((prevState) => produce(prevState, (draft) => {
  //         draft.steps[index]!.checkStatus = 'ready';
  //       }));
  //     } catch (err) {
  //       try {
  //         setState((prevState) => produce(prevState, (draft) => {
  //           draft.steps[index]!.checkStatus = 'install';
  //         }));
  //         await step.install();
  //       } catch (err2) {
  //         aborted = true;
  //         setState((prevState) => produce(prevState, (draft) => {
  //           draft.steps[index]!.checkStatus = 'installFailed';
  //           draft.steps[index]!.error = err2 as Error;
  //         }));
  //       }
  //       // FIXME: if check failed, try install and set status?
  //       // if install success, and if it requires restart, set current step restarted required, set other steps "aborted"
  //     }
  //   }
  //   setState((prevState) => ({ ...prevState, pending: false, ready: !aborted }));
  // }, [state.pending]);
  const value = useMemo(() => ({
    ...state,
    currentStep: state.currentStepIndex != null
      ? state.steps[state.currentStepIndex]
      : undefined,
    performCheck,
    // performInstall: process,
  }), [state, performCheck]);
  useEffect(() => {
    // FIXME: subscribe event for output sink
  }, []);
  return value;
}
