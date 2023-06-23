import './bootstrap';
import { invoke } from '@tauri-apps/api';
import ReactDOM from 'react-dom/client';
import { FocusStyleManager } from '@blueprintjs/core';
import { setupListenerHook } from './utils/logger';
import LogPage from './pages/Log';
import './styles.css';

await setupListenerHook();

FocusStyleManager.onlyShowFocusOnTabs();

ReactDOM.createRoot(document.getElementById('__root__') as HTMLElement).render(
  <LogPage />,
);

invoke('window_ready', { name: 'log' });
