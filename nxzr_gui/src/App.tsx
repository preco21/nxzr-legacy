import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

import reactLogo from './assets/react.svg';

import './App.css';

function App(): React.ReactElement {
  const [greetMsg, setGreetMsg] = useState('');
  const [name, setName] = useState('');

  const greet = async (): Promise<void> => {
    setGreetMsg(await invoke('greet', { name }));
  };

  return (
    <div className="container">
      <h1>Welcome to Tauri!</h1>

      <div className="row">
        <a href="https://vitejs.dev" target="_blank" rel="noreferrer">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank" rel="noreferrer">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://reactjs.org" target="_blank" rel="noreferrer">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>

      <p>Click on the Tauri, Vite, and React logos to learn more.</p>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          id="greet-input"
          placeholder="Enter a name..."
          onChange={(e) => setName(e.currentTarget.value)}
        />
        <button type="submit">Greet</button>
      </form>

      <p>{greetMsg}</p>
    </div>
  );
}

export default App;
