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
  const [selectedAdapterId, setSelectedAdapterId] = useState<string | undefined>();
  const handleClose = (): void => {
    if (adapterManager.pending) {
      return;
    }
    setError(undefined);
    setSelectedAdapterId(undefined);
    onClose();
  };
  const handleAttach = async (): Promise<void> => {
    if (selectedAdapterId == null) {
      return;
    }
    try {
      await adapterManager.attachAdapter(selectedAdapterId);
      onClose();
      setSelectedAdapterId(undefined);
    } catch (err) {
      setError((err as Error).message);
    }
  };
  const handleRefresh = async (): Promise<void> => {
    setError(undefined);
    await adapterManager.refreshAdapterList();
    setSelectedAdapterId(undefined);
  };
  useEffect(() => {
    const currentAdapter = adapterManager.selectedAdapter;
    if (isOpen && currentAdapter != null) {
      setSelectedAdapterId(currentAdapter.id);
    }
  }, [isOpen, adapterManager.selectedAdapter]);
  const pending = adapterManager.pending;
  return (
    <Container className={className}>
      <Dialog
        className="bp5-dark"
        title="Choose a Bluetooth Adapter"
        icon="signal-search"
        isOpen={isOpen}
        canEscapeKeyClose={!pending}
        canOutsideClickClose={!pending}
        isCloseButtonShown={!pending}
        autoFocus
        onClose={handleClose}
      >
        <DialogBody>
          <AdapterSelect
            adapters={adapterManager.adapters}
            value={selectedAdapterId}
            disabled={pending}
            onSelect={(id) => setSelectedAdapterId(id)}
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
                  selectedAdapterId == null ||
                  selectedAdapterId === adapterManager.selectedAdapter?.id
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

const ErrorCallout = styled(Callout)`
  margin-top: 8px;
`;
