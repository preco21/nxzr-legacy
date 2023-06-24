import { produce } from 'immer';
import { useCallback, useMemo, useState } from 'react';
import {
  check1SetupInstalled,
  check2Wslconfig,
  check3AgentRegistered,
  install1ProgramSetup,
  install2EnsureWslconfig,
  install3RegisterAgent,
} from '../../common/commands';
import { sleep } from '../../utils/promise';

interface SetupStep {
  name: string;
  description: string;
  rebootRequired: boolean;
  check: () => Promise<void>;
  install: () => Promise<void>;
}

const SETUP_STEPS: SetupStep[] = [
  {
    name: 'WSL & Program Requirements',
    description: 'This step will install all the necessary components of the program on your computer.',
    rebootRequired: true,
    check: () => check1SetupInstalled(),
    install: () => install1ProgramSetup(),
  },
  {
    name: 'WSL Global Configuration',
    description: 'This step will ensure that WSL is configured correctly on your computer.',
    rebootRequired: false,
    check: () => check2Wslconfig(),
    install: () => install2EnsureWslconfig(),
  },
  {
    name: 'Agent Registration',
    description: 'This step will register the server daemon with the WSL instance.',
    rebootRequired: false,
    check: () => check3AgentRegistered(),
    install: () => install3RegisterAgent(),
  },
];

export interface UseSetupGuardOptions {
  onCheckComplete?: () => void;
  onRebootRequest?: () => void;
}

export interface UseSetupGuardState {
  pending: boolean;
  inInstall: boolean;
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
  performInstall: () => void;
}

export interface StepDisplay {
  name: string;
  description: string;
  rebootRequired: boolean;
  status: 'none' | 'wait' | 'check' | 'checkFailed' | 'install' | 'installFailed' | 'rebootRequired' | 'ready' | 'skipped';
  error?: Error;
}

export function useSetupGuard(options?: UseSetupGuardOptions): UseSetupGuard {
  const [state, setState] = useState<UseSetupGuardState>(() => ({
    pending: false,
    inInstall: false,
    ready: false,
    steps: SETUP_STEPS.map((step) => ({
      name: step.name,
      description: step.description,
      rebootRequired: step.rebootRequired ?? false,
      status: 'none',
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
        step.status = 'wait';
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
            draft.steps[index]!.status = 'skipped';
          }));
          continue;
        }
        setState((prevState) => produce(prevState, (draft) => {
          draft.currentStepIndex = index;
          draft.steps[index]!.status = 'check';
        }));
        await step.check();
        setState((prevState) => produce(prevState, (draft) => {
          draft.steps[index]!.status = 'ready';
        }));
      } catch (err) {
        aborted = true;
        setState((prevState) => produce(prevState, (draft) => {
          draft.steps[index]!.status = 'checkFailed';
          draft.steps[index]!.error = new Error(err as string);
        }));
      }
    }
    // Wait for a moment to allow the UI to update.
    await sleep(100);
    setState((prevState) => produce(prevState, (draft) => {
      draft.pending = false;
      draft.ready = !aborted;
    }));
    if (!aborted) {
      options?.onCheckComplete?.();
    }
  }, [state.pending]);
  const performInstall = useCallback(async () => {
    if (state.pending) {
      return;
    }
    setState((prevState) => produce(prevState, (draft) => {
      draft.pending = true;
      draft.inInstall = true;
      for (const step of draft.steps) {
        step.status = 'wait';
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
            draft.steps[index]!.status = 'skipped';
          }));
          continue;
        }
        setState((prevState) => produce(prevState, (draft) => {
          draft.currentStepIndex = index;
          draft.steps[index]!.status = 'check';
        }));
        await step.check();
        setState((prevState) => produce(prevState, (draft) => {
          draft.steps[index]!.status = 'ready';
        }));
      } catch {
        try {
          setState((prevState) => produce(prevState, (draft) => {
            draft.steps[index]!.status = 'install';
          }));
          await step.install();
          await new Promise((r) => setTimeout(r, 1000));
          if (step.rebootRequired) {
            aborted = true;
            setState((prevState) => produce(prevState, (draft) => {
              draft.steps[index]!.status = 'rebootRequired';
            }));
            options?.onRebootRequest?.();
          } else {
            setState((prevState) => produce(prevState, (draft) => {
              draft.steps[index]!.status = 'check';
            }));
            // Check the step again to ensure it was installed correctly.
            await step.check();
            setState((prevState) => produce(prevState, (draft) => {
              draft.steps[index]!.status = 'ready';
            }));
          }
        } catch (err) {
          aborted = true;
          setState((prevState) => produce(prevState, (draft) => {
            draft.steps[index]!.status = 'installFailed';
            draft.steps[index]!.error = new Error(err as string);
          }));
        }
      }
    }
    // Wait for a moment to allow the UI to update.
    await sleep(100);
    setState((prevState) => produce(prevState, (draft) => {
      draft.pending = false;
      draft.inInstall = false;
      draft.ready = !aborted;
    }));
    if (!aborted) {
      options?.onCheckComplete?.();
    }
  }, [state.pending]);
  const value = useMemo(() => ({
    ...state,
    currentStep: state.currentStepIndex != null
      ? state.steps[state.currentStepIndex]
      : undefined,
    performCheck,
    performInstall,
  }), [state, performCheck]);
  return value;
}
