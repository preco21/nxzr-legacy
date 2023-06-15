import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { MainContainer } from '../components/MainContainer';

function LogPage(): React.ReactElement {
  return (
    <MainContainer>
      테스트 로거
    </MainContainer>
  );
}

export default LogPage;
