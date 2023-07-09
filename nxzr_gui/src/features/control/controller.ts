import { listenRawMouseMove, preventCursorEscape } from '../../utils/control';

export type ButtonKey =
  | 'Y'
  | 'X'
  | 'B'
  | 'A'
  | 'R'
  | 'Zr'
  | 'Minus'
  | 'Plus'
  | 'RStick'
  | 'LStick'
  | 'Home'
  | 'Capture'
  | 'Down'
  | 'Up'
  | 'Right'
  | 'Left'
  // | 'Sr'
  // | 'Sl'
  | 'L'
  | 'Zl';

export interface ControllerConfig {
  button?: {
    keyboardKey: KeyboardEvent['key'];
    button: ButtonKey;
  }[];
  stick?: {
    keyboardKey: KeyboardEvent['key'];
    stick: 'left' | 'right';
    direction: 'up' | 'down' | 'left' | 'right';
    sensitivity?: number;
  }[];
  imu?: {
    sensitivity?: number;
    deadzone?: number;
    invert?: boolean;
  };
}

export interface InputUpdatePayload {
  buttonMap: Record<ButtonKey, boolean>;
  leftStickPosition: Position;
  rightStickPosition: Position;
  imuPosition: Position;
}

export interface Position {
  x: number;
  y: number;
}

export type StickType = 'left' | 'right';
export type StickDirection = 'up' | 'down' | 'left' | 'right';

export class ControllerEventManager {
  private readonly buttonMap: Map<KeyboardEvent['key'], ButtonKey>;
  private readonly stickMap: Map<KeyboardEvent['key'], {
    stick: StickType;
    direction: StickDirection;
    sensitivity: number;
  }>;
  private readonly imuOptions: {
    sensitivity: number;
    deadzone: number;
    invert: boolean;
  };

  private buttonState: Record<ButtonKey, boolean> = {
    Y: false,
    X: false,
    B: false,
    A: false,
    R: false,
    Zr: false,
    Minus: false,
    Plus: false,
    RStick: false,
    LStick: false,
    Home: false,
    Capture: false,
    Down: false,
    Up: false,
    Right: false,
    Left: false,
    // Sr: false,
    // Sl: false,
    L: false,
    Zl: false,
  };
  // FIXME: Currently only supports direction but no sensitivity.
  private leftStickState: StickDirection[] = [];
  private rightStickState: StickDirection[] = [];
  private imuState: Position = {
    x: 0,
    y: 0,
  };

  private listeners: Set<(payload: InputUpdatePayload) => void> = new Set();

  constructor(config: ControllerConfig) {
    this.buttonMap = new Map(config.button?.map((button) => [button.keyboardKey, button.button]));
    this.stickMap = new Map(config.stick?.map((stick) => [stick.keyboardKey, {
      stick: stick.stick,
      direction: stick.direction,
      sensitivity: stick.sensitivity ?? 1,
    }]));
    this.imuOptions = {
      sensitivity: config.imu?.sensitivity ?? 1,
      deadzone: config.imu?.deadzone ?? 0,
      invert: config.imu?.invert ?? false,
    };
  }

  public async init(): Promise<void> {
    await preventCursorEscape();
    await listenRawMouseMove((pos) => {
      this.imuState.x = convertNumberToU16(pos.x);
      this.imuState.y = convertNumberToU16(pos.y);
    });
    const dispatchChanges = (): void => {
      // FIXME:
      // States can overlap each other; this means, the last element in the state will win.
      const leftStickPosition = stickDirectionsToPosition(this.leftStickState);
      const rightStickPosition = stickDirectionsToPosition(this.rightStickState);
      this.emitUpdate({
        buttonMap: this.buttonState,
        leftStickPosition,
        rightStickPosition,
        imuPosition: this.imuState,
      });
    };
    // FIXME: hard-coded state, move this into dynamic config.
    window.addEventListener('mousedown', (e) => {
      switch (e.button) {
        case 0: {
          this.buttonState['Zr'] = true;
          break;
        }
        case 1: {
          this.buttonState['R'] = true;
          break;
        }
        default: {
          // noop
        }
      }
    });
    window.addEventListener('mouseup', (e) => {
      switch (e.button) {
        case 0: {
          this.buttonState['Zr'] = false;
          break;
        }
        case 1: {
          this.buttonState['R'] = false;
          break;
        }
        default: {
          // noop
        }
      }
    });
    window.addEventListener('keydown', (e) => {
      if (this.buttonMap.has(e.key)) {
        const button = this.buttonMap.get(e.key)!;
        this.buttonState[button] = true;
      }
      if (this.stickMap.has(e.key)) {
        const stick = this.stickMap.get(e.key)!;
        switch (stick.stick) {
          case 'left': {
            this.leftStickState.push(stick.direction);
            break;
          }
          case 'right': {
            this.rightStickState.push(stick.direction);
            break;
          }
          default: {
            // noop
          }
        }
      }
      dispatchChanges();
    });
    window.addEventListener('keyup', (e) => {
      if (this.buttonMap.has(e.key)) {
        const button = this.buttonMap.get(e.key)!;
        this.buttonState[button] = false;
      }
      if (this.stickMap.has(e.key)) {
        const stick = this.stickMap.get(e.key)!;
        switch (stick.stick) {
          case 'left': {
            this.leftStickState = this.leftStickState.filter((dir) => dir !== stick.direction);
            break;
          }
          case 'right': {
            this.rightStickState = this.rightStickState.filter((dir) => dir !== stick.direction);
            break;
          }
          default: {
            // noop
          }
        }
      }
      dispatchChanges();
    });
  }

  private emitUpdate(update: InputUpdatePayload): void {
    for (const listener of this.listeners) {
      listener(update);
    }
  }

  public onUpdate(listener: (payload: InputUpdatePayload) => void): (() => void) {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }
}

const MAX_U16 = 0xFFFF;
const OFFSET = Math.floor(MAX_U16 / 2);
function convertNumberToU16(input: number): number {
  return Math.floor((input + OFFSET) & MAX_U16);
}

function stickDirectionsToPosition(directions: StickDirection[]): Position {
  let x = 0;
  let y = 0;
  for (const direction of directions) {
    switch (direction) {
      case 'up': {
        y = 1;
        break;
      }
      case 'down': {
        y = -1;
        break;
      }
      case 'left': {
        x = -1;
        break;
      }
      case 'right': {
        x = 1;
        break;
      }
      default: {
        // noop
      }
    }
  }
  return { x, y };
}
