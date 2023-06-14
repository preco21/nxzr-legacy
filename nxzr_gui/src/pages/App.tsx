import React, { useState } from 'react';
import { Button } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/tauri';
import { MainContainer } from '../components/MainContainer';
import './App.css';
import { css } from 'styled-components';
import { TitleBar } from '../components/TitleBar';
import { Header } from '../components/Header';

function App(): React.ReactElement {
  const [greetMsg, setGreetMsg] = useState('');
  const [name, setName] = useState('');

  const greet = async (): Promise<void> => {
    setGreetMsg(await invoke('greet', { name }));
  };

  return (
    <MainContainer>
      <TitleBar />
      <Header />
    </MainContainer>
  );
}

export default App;
