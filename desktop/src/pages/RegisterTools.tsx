import { useState } from 'react';
import { ReceiptText, Wallet } from 'lucide-react';
import { api, type Bootstrap, type CashOverview } from '@/lib/api';
import { useResource } from '@/lib/useResource';
import { money, parseScaled, today, dateTime } from '@/lib/utils';
import { Button, Busy, Empty, Failure, Field, Modal, useOperation } from '@/components/common';
import { DocumentDialog } from '@/components/DocumentViewer';

export function RegisterRecent({
  data,
  onClose,
  onHistory,
}: {
  data: Bootstrap;
  onClose: () => void;
  onHistory: () => void;
}) {
  const sales = useResource(() => api('sales', { from: today(), to: today(), page: 0 }), []);
  const [document, setDocument] = useState<number | null>(null);
  if (document !== null)
    return (
      <DocumentDialog
        data={data}
        request={{ kind: 'sale', id: document }}
        onClose={() => setDocument(null)}
      />
    );
  return (
    <Modal
      open
      onClose={onClose}
      title="Ventas de hoy"
      description="Revisa o reimprime un folio. El ticket en curso se conserva."
    >
      <Failure error={sales.error} retry={sales.reload} />
      {sales.busy ? (
        <Busy />
      ) : sales.data?.length ? (
        <div className="register-recent">
          {sales.data.slice(0, 20).map((s) => (
            <button key={s.id} onClick={() => setDocument(s.id)}>
              <ReceiptText size={19} />
              <span>
                <strong>
                  {s.folio} · {money(s.totalCents)}
                </strong>
                <small>
                  {s.customerName} · {dateTime(s.createdAt)}
                </small>
              </span>
              <span>Ver ticket</span>
            </button>
          ))}
        </div>
      ) : (
        <Empty title="Tu primera venta está por llegar." />
      )}
      <div className="dialog-actions">
        <Button variant="outline" onClick={onHistory}>
          Historial completo
        </Button>
        <Button onClick={onClose}>Seguir vendiendo</Button>
      </div>
    </Modal>
  );
}
export function RegisterCash({
  cash,
  onClose,
  onSaved,
  onManage,
}: {
  cash: CashOverview | null;
  onClose: () => void;
  onSaved: (cash: CashOverview) => void;
  onManage: () => void;
}) {
  const [amount, setAmount] = useState('');
  const [requestId] = useState(() => crypto.randomUUID());
  const op = useOperation();
  return (
    <Modal
      open
      onClose={() => {
        if (!op.busy) onClose();
      }}
      title={cash?.open ? 'Tu caja está abierta' : 'Abre la caja y comienza'}
      description={
        cash?.open
          ? 'Puedes continuar con el cliente sin salir del mostrador.'
          : 'Indica el fondo inicial. Esta apertura queda registrada a tu nombre.'
      }
    >
      {cash?.open ? (
        <>
          <div className="register-cash">
            <Wallet size={28} />
            <div>
              <strong>{cash.open.folio}</strong>
              <p>Fondo inicial: {money(cash.open.openingCents)}</p>
            </div>
          </div>
          <div className="dialog-actions">
            <Button variant="outline" onClick={onManage}>
              Ver movimientos
            </Button>
            <Button onClick={onClose}>Seguir vendiendo</Button>
          </div>
        </>
      ) : (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void op.run(
              () => api('cashOpen', { requestId, openingCents: parseScaled(amount) }),
              onSaved,
            );
          }}
        >
          <Field label="Fondo inicial (MXN)">
            <input
              autoFocus
              required
              type="number"
              min="0"
              max="1000000000"
              step="0.01"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              disabled={op.busy}
            />
          </Field>
          <Failure error={op.error} />
          <div className="dialog-actions">
            <Button type="button" variant="outline" disabled={op.busy} onClick={onClose}>
              Volver
            </Button>
            <Button type="submit" disabled={op.busy}>
              {op.busy ? 'Abriendo…' : 'Abrir caja'}
            </Button>
          </div>
        </form>
      )}
    </Modal>
  );
}
