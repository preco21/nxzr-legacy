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
  | 'Sr'
  | 'Sl'
  | 'L'
  | 'Zl';

export interface KeyMapButton {
  key: KeyboardEvent['key'];
}

export interface KeyMapStick {
  stick: 'left' | 'right';
  direction: 'up' | 'down' | 'left' | 'right';
  sensitivity?: number;
}

export interface ImuOptions {
  sensitivity?: number;
  deadzone?: number;
  invert?: boolean;
}

export interface ControllerConfig {
  button: KeyMapButton[];
  stick: KeyMapStick[];
  imu: ImuOptions;
}

export interface ControllerInputPayload {
  buttons: {
    key: ButtonKey;
    action: 'up' | 'down' | 'press';
  }[];
  stick: {
    left: {
      x: number;
      y: number;
    };
    right: {
      x: number;
      y: number;
    };
  };
  imu: {
    x: number;
    y: number;
  };
}

export class ControllerEventManager {
  private readonly buttonMap: Map<ButtonKey, KeyboardEvent['key']>;
  private readonly stickMap: Map<'left' | 'right', Map<'up' | 'down' | 'left' | 'right', KeyboardEvent['key']>>;
  private readonly imuOptions: ImuOptions;

  private buttonState: [ButtonKey, KeyMapButton][];
  private stickState: [ButtonKey, KeyMapStick][];
  private imuState: {
    x: number;
    y: number;
  };

  private listeners: Set<(payload: ControllerInputPayload) => void> = new Set();

  constructor(config: ControllerConfig) {

  }

  public onUpdate(listener: (payload: ControllerInputPayload) => void): (() => void) {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }
}
