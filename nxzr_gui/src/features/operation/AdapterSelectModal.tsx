import React, { useEffect, useState } from 'react';
import styled from 'styled-components';
import { Button, Callout, Dialog, DialogBody, DialogFooter } from '@blueprintjs/core';
import { UseAdapterManager } from './useAdapterManager';
import { AdapterSelect } from './AdapterSelect';

export interface AdapterSelectModalProps {
  className?: string;
  isOpen?: boolean;
  adapterManager: UseAdapterManager;
  onClose: () => void;
}

export function AdapterSelectModal(props: AdapterSelectModalProps): React.ReactElement {
  const { className, isOpen, adapterManager, onClose } = props;
  const [error, setError] = useState<string | undefined>(undefined);
  const [selectedAdapter, setSelectedAdapter] = useState<string | undefined>(undefined);
  const handleClose = (): void => {
    if (adapterManager.pending) {
      return;
    }
    setError(undefined);
    setSelectedAdapter(undefined);
    onClose();
  };
  const handleAttach = async (): Promise<void> => {
    if (selectedAdapter == null) {
      return;
    }
    try {
      const currentAdapter = adapterManager.selectedAdapter;
      if (currentAdapter != null) {
        await adapterManager.detachAdapter(currentAdapter.id);
      }
      await adapterManager.attachAdapter(selectedAdapter);
      onClose();
      setSelectedAdapter(undefined);
    } catch (err) {
      setError((err as Error).message);
    }
  };
  const handleRefresh = async (): Promise<void> => {
    setError(undefined);
    setSelectedAdapter(undefined);
    await adapterManager.refreshAdapterList();
  };
  useEffect(() => {
    const currentAdapter = adapterManager.selectedAdapter;
    if (currentAdapter != null) {
      setSelectedAdapter(currentAdapter.id);
    }
  }, [adapterManager.selectedAdapter]);
  const pending = adapterManager.pending;
  return (
    <Container className={className}>
      <Dialog
        className="bp5-dark"
        title="Bluetooth Adapter Select"
        icon="signal-search"
        isOpen={isOpen}
        canEscapeKeyClose={!pending}
        canOutsideClickClose={!pending}
        isCloseButtonShown={!pending}
        autoFocus
        onClose={handleClose}
      >
        <DialogBody>
          <Description>Please select the appropriate Bluetooth adapter.</Description>
          <AdapterSelect
            adapters={adapterManager.adapters}
            value={selectedAdapter}
            disabled={pending}
            onSelect={(id) => setSelectedAdapter(id)}
            onRefresh={handleRefresh}
          />
          {error && (
            <ErrorCallout intent="danger" title="Error">
              {error}
            </ErrorCallout>
          )}
        </DialogBody>
        <DialogFooter
          actions={(
            <>
              <Button
                text="Close"
                disabled={pending}
                onClick={handleClose}
              />
              <Button
                intent="primary"
                text="Confirm"
                loading={pending}
                disabled={(
                  pending ||
                  selectedAdapter == null ||
                  selectedAdapter === adapterManager.selectedAdapter?.id
                )}
                onClick={handleAttach}
              />
            </>
          )}
        />
      </Dialog>
    </Container>
  );
}

const Container = styled.div`
`;

const Description = styled.div`
  margin-bottom: 8px;
`;

const ErrorCallout = styled(Callout)`
  margin-top: 8px;
`;
