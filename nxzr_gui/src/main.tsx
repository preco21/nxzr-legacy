import './bootstrap';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { FocusStyleManager } from '@blueprintjs/core';
import AppPage from './pages/App';
import './styles.css';

FocusStyleManager.onlyShowFocusOnTabs();

ReactDOM.createRoot(document.getElementById('__root__') as HTMLElement).render(
  <React.StrictMode>
    <AppPage />
  </React.StrictMode>,
);
