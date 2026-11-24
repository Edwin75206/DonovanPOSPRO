import { useState } from 'react';
import { Trash2 } from 'lucide-react';
import { Button, Failure, Field, Modal, useOperation } from './common';

export function DeleteRecordDialog({
  label,
  name,
  description,
  requiresReason = false,
  onDelete,
  onClose,
  onDeleted,
}: {
  label: string;
  name: string;
  description?: string;
  requiresReason?: boolean;
  onDelete: (reason: string) => Promise<unknown>;
  onClose: () => void;
  onDeleted: () => void;
}) {
  const op = useOperation();
  const [reason, setReason] = useState('');
  return (
    <Modal
      open
      title={label}
      description={description ?? 'La eliminación es definitiva. Se registra quién la realizó.'}
      onClose={() => {
        if (!op.busy) onClose();
      }}
    >
      <form
        className="record-form"
        onSubmit={(event) => {
          event.preventDefault();
          if (!requiresReason || reason.trim()) void op.run(() => onDelete(reason.trim()), onDeleted);
        }}
      >
        <p><strong>{name}</strong></p>
        {requiresReason && (
          <Field label="Motivo">
            <textarea required maxLength={300} value={reason} disabled={op.busy}
              onChange={(event) => setReason(event.target.value)} />
          </Field>
        )}
        <Failure error={op.error} />
        <div className="dialog-actions">
          <Button type="button" variant="outline" disabled={op.busy} onClick={onClose}>Cancelar</Button>
          <Button type="submit" variant="destructive" disabled={op.busy || (requiresReason && !reason.trim())}>
            <Trash2 size={16} /> {op.busy ? 'Eliminando…' : label}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
