import './bootstrap';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { FocusStyleManager } from '@blueprintjs/core';
import LogPage from './pages/Log';
import './styles.css';

FocusStyleManager.onlyShowFocusOnTabs();

ReactDOM.createRoot(document.getElementById('__root__') as HTMLElement).render(
  <React.StrictMode>
    <LogPage />
  </React.StrictMode>,
);
