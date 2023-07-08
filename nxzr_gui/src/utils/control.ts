import { listen } from '@tauri-apps/api/event';
import { LogicalPosition, PhysicalPosition, appWindow } from '@tauri-apps/api/window';

export interface MousePosition {
  x: number;
  y: number;
}

export async function preventCursorEscape(
  onMouseMove?: (position: MousePosition) => void,
): Promise<() => Promise<void>> {
  const resizable = await appWindow.isResizable();
  if (resizable) {
    await appWindow.setResizable(false);
  }
  const forceFocus = async (): Promise<void> => {
    // await appWindow.setFocus();
    await appWindow.setCursorGrab(true);
    // await appWindow.setCursorVisible(false);
  };
  const unlistenFocusChange = await appWindow.onFocusChanged(async ({ payload: focused }) => {
    if (!focused) {
      await forceFocus();
    }
  });
  const resetCursorPos = async (): Promise<void> => {
    const x = window.innerWidth / 2;
    const y = window.innerHeight / 2;
    await appWindow.setCursorPosition(new PhysicalPosition(x, y));
  };
  const handleMouseMove = async (e: MouseEvent): Promise<void> => {
    await resetCursorPos();
  };
  const handleMouseLeave = async (): Promise<void> => {
    await resetCursorPos();
  };
  await resetCursorPos();
  await forceFocus();
  const unlistenRawMouseMove = await listen('raw_input:mousemove', async (event) => {
    const pos = event.payload as MousePosition;
    onMouseMove?.(pos);
    await resetCursorPos();
  });
  window.addEventListener('mousemove', handleMouseMove);
  document.documentElement.addEventListener('mouseleave', handleMouseLeave);
  return async () => {
    document.documentElement.removeEventListener('mouseleave', handleMouseLeave);
    window.removeEventListener('mousemove', handleMouseMove);
    await appWindow.setCursorGrab(false);
    unlistenFocusChange();
    unlistenRawMouseMove();
    if (resizable) {
      await appWindow.setResizable(true);
    }
  };
}
