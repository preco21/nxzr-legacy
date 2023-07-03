import { produce } from 'immer';
import { useCallback, useState } from 'react';
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
  canRefresh: boolean;
  check: () => Promise<void>;
  install: () => Promise<void>;
}

const SETUP_STEPS: SetupStep[] = [
  {
    name: 'WSL & Program Requirements',
    description: 'This step will install all the necessary components of the program on your computer.',
    rebootRequired: true,
    canRefresh: false,
    check: () => check1SetupInstalled(),
    install: () => install1ProgramSetup(),
  },
  {
    name: 'WSL Global Configuration',
    description: 'This step will ensure that WSL is configured correctly on your computer.',
    rebootRequired: false,
    canRefresh: true,
    check: () => check2Wslconfig(),
    install: () => install2EnsureWslconfig(),
  },
  {
    name: 'Agent Registration',
    description: 'This step will register the server daemon with the WSL instance.',
    rebootRequired: false,
    canRefresh: true,
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
  isReady: boolean;
  installRequired: boolean;
  steps: StepDisplay[];
  currentStepIndex?: number;
}

export interface UseSetupGuard {
  pending: boolean;
  isReady: boolean;
  installRequired: boolean;
  steps: StepDisplay[];
  currentStep?: StepDisplay;
  performCheck: () => void;
  performInstall: (forceReinstall?: boolean) => void;
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
    isReady: false,
    installRequired: false,
    steps: SETUP_STEPS.map((step) => ({
      name: step.name,
      description: step.description,
      rebootRequired: step.rebootRequired ?? false,
      status: 'none',
      error: undefined,
    })),
    currentStepIndex: undefined,
  }));
  const performCheck = useCallback(async () => {
    if (state.pending) {
      return;
    }
    setState((prevState) => produce(prevState, (draft) => {
      draft.pending = true;
      draft.installRequired = false;
      for (const step of draft.steps) {
        step.status = 'wait';
        step.error = undefined;
      }
      draft.currentStepIndex = undefined;
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
          draft.steps[index]!.error = err as Error;
        }));
      }
    }
    // Wait for a moment to allow the UI to update.
    await sleep(100);
    setState((prevState) => produce(prevState, (draft) => {
      draft.pending = false;
      draft.isReady = !aborted;
      draft.installRequired = aborted;
    }));
    if (!aborted) {
      options?.onCheckComplete?.();
    }
  }, [state.pending, options?.onCheckComplete]);
  const performInstall = useCallback(async (forceReinstall?: boolean) => {
    if (state.pending) {
      return;
    }
    setState((prevState) => produce(prevState, (draft) => {
      draft.pending = true;
      draft.inInstall = true;
      draft.installRequired = false;
      for (const step of draft.steps) {
        step.status = 'wait';
        step.error = undefined;
      }
      draft.currentStepIndex = undefined;
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
        if (forceReinstall && !step.rebootRequired && step.canRefresh) {
          throw new Error('Performing force reinstall.');
        } else {
          await step.check();
        }
        setState((prevState) => produce(prevState, (draft) => {
          draft.steps[index]!.status = 'ready';
        }));
      } catch {
        try {
          setState((prevState) => produce(prevState, (draft) => {
            draft.steps[index]!.status = 'install';
          }));
          await step.install();
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
            draft.steps[index]!.error = err as Error;
          }));
        }
      }
    }
    // Wait for a moment to allow the UI to update.
    await sleep(100);
    setState((prevState) => produce(prevState, (draft) => {
      draft.pending = false;
      draft.inInstall = false;
      draft.isReady = !aborted;
      draft.installRequired = aborted;
    }));
    if (!aborted) {
      options?.onCheckComplete?.();
    }
  }, [state.pending, options?.onCheckComplete]);
  return {
    ...state,
    currentStep: state.currentStepIndex != null
      ? state.steps[state.currentStepIndex]
      : undefined,
    performCheck,
    performInstall,
  };
}
