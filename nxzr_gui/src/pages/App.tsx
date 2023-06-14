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
      <div
        css={css`
          display: grid;
          grid-template-areas:
            "content content sidebar"
            "slider1 slider2 sidebar";
          grid-template-rows: 130px 300px;
          grid-template-columns: 1fr 1fr 1fr;
        `}
      >
        <div css={css`grid-area: content; background-color: #009688`}>게임패드 미리보기</div>
        <div css={css`grid-area: sidebar; background-color: blueviolet`}>키 바인딩 테이블</div>
        <div css={css`grid-area: slider1; background-color: cornflowerblue`}>자이로 설정</div>
        <div css={css`grid-area: slider2; background-color:maroon`}>스틱 설정</div>
      </div>
    </MainContainer>
  );
}

export default App;
