import './setup';
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles.css';

ReactDOM.createRoot(document.getElementById('__root__') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
