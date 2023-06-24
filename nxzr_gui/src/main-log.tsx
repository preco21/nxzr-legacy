import './bootstrap';
import ReactDOM from 'react-dom/client';
import { FocusStyleManager } from '@blueprintjs/core';
import { windowReady } from './common/commands';
import { setupListenerHook } from './utils/logger';
import LogPage from './pages/Log';
import './styles.css';

await setupListenerHook();
FocusStyleManager.onlyShowFocusOnTabs();

ReactDOM.createRoot(document.getElementById('__root__') as HTMLElement).render(
  <LogPage />,
);

windowReady('log');
