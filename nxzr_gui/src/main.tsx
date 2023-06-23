import './bootstrap';
import { invoke } from '@tauri-apps/api';
import ReactDOM from 'react-dom/client';
import { FocusStyleManager } from '@blueprintjs/core';
import AppPage from './pages/App';
import './styles.css';

FocusStyleManager.onlyShowFocusOnTabs();

ReactDOM.createRoot(document.getElementById('__root__') as HTMLElement).render(
  <AppPage />,
);

invoke('window_ready', { name: 'main' });
