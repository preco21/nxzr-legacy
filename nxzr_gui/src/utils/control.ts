import { UnlistenFn, listen } from '@tauri-apps/api/event';
import { LogicalPosition, appWindow } from '@tauri-apps/api/window';

export interface MousePosition {
  x: number;
  y: number;
}

export async function listenRawMouseMove(
  onMouseMove: (position: MousePosition) => void,
): Promise<UnlistenFn> {
  const unlisten = await listen('raw_input:mousemove', (event) => {
    const position = event.payload as MousePosition;
    onMouseMove(position);
  });
  return unlisten;
}

export async function preventCursorEscape(): Promise<() => Promise<void>> {
  const resizable = await appWindow.isResizable();
  if (resizable) {
    await appWindow.setResizable(false);
  }
  const forceFocus = async (): Promise<void> => {
    // FIXME: enable after implementing escape key
    // await appWindow.setFocus();
    await appWindow.setCursorGrab(true);
  };
  const unlistenFocusChange = await appWindow.onFocusChanged(async ({ payload: focused }) => {
    if (!focused) {
      await forceFocus();
    }
  });
  const handleResetCursorPos = async (): Promise<void> => {
    const x = window.innerWidth / 2;
    const y = window.innerHeight / 2;
    await appWindow.setCursorPosition(new LogicalPosition(x, y));
  };
  await handleResetCursorPos();
  await forceFocus();
  window.addEventListener('mousemove', handleResetCursorPos);
  document.documentElement.addEventListener('mouseleave', handleResetCursorPos);
  return async () => {
    document.documentElement.removeEventListener('mouseleave', handleResetCursorPos);
    window.removeEventListener('mousemove', handleResetCursorPos);
    await appWindow.setCursorGrab(false);
    unlistenFocusChange();
    if (resizable) {
      await appWindow.setResizable(true);
    }
  };
}
