import './bootstrap';
import ReactDOM from 'react-dom/client';
import { FocusStyleManager } from '@blueprintjs/core';
import { windowReady } from './common/commands';
import AppPage from './pages/App';
import './styles.css';

FocusStyleManager.onlyShowFocusOnTabs();

ReactDOM.createRoot(document.getElementById('__root__') as HTMLElement).render(
  <AppPage />,
);

windowReady('main');
