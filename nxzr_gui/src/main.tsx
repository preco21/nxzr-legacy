import './bootstrap';
import ReactDOM from 'react-dom/client';
import { FocusStyleManager } from '@blueprintjs/core';
import { windowReady } from './common/commands';
import AppPage from './pages/App';
import { AlertManagerProvider } from './components/AlertManager';
import './styles.css';

FocusStyleManager.onlyShowFocusOnTabs();

ReactDOM.createRoot(document.getElementById('__root__') as HTMLElement).render(
  <AlertManagerProvider>
    <AppPage />
  </AlertManagerProvider>,
);

windowReady('main');
