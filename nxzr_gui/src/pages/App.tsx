import React, { useState } from 'react';
import { Button } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/tauri';
import { MainContainer } from '../components/MainContainer';
import './App.css';
import { css } from 'styled-components';
import { TitleBar } from '../components/TitleBar';

function App(): React.ReactElement {
  const [greetMsg, setGreetMsg] = useState('');
  const [name, setName] = useState('');

  const greet = async (): Promise<void> => {
    setGreetMsg(await invoke('greet', { name }));
  };

  return (
    <MainContainer>
      <TitleBar />

      <div css={css`background-color: purple;`}>
        <Button>Hello World</Button>
        <Button>Hello World</Button>
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
    </MainContainer>
  );
}

export default App;
