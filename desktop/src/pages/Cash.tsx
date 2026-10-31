import { useEffect, useState } from 'react';
import {
  Wallet,
  ArrowDownLeft,
  ArrowUpRight,
  Check,
  Plus,
  LockKeyhole,
  RefreshCw,
} from 'lucide-react';
import { api, normalizeError, type CashOverview, type UiFailure } from '@/lib/api';
import { Button, PageTitle, Field, Modal, Failure, useOperation, Empty } from '@/components/common';
import { money, parseScaled, dateTime } from '@/lib/utils';
import { DataGrid } from '@/components/DataGrid';
export function Cash() {
  const [data, setData] = useState<CashOverview | null>(null);
  const [modal, setModal] = useState<'open' | 'move' | 'close' | null>(null);
  const [amount, setAmount] = useState('');
  const [reason, setReason] = useState('');
  const [requestId, setRequestId] = useState(() => crypto.randomUUID());
  const [kind, setKind] = useState<'in' | 'out'>('in');
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<UiFailure | null>(null);
  const op = useOperation();
  const refresh = () => {
    setLoading(true);
    setLoadError(null);
    void api('cashOverview')
      .then(setData)
      .catch((error) => {
        setLoadError(normalizeError(error));
        setData(null);
      })
      .finally(() => setLoading(false));
  };
  useEffect(refresh, []);
  const open = (action: typeof modal) => {
    setModal(action);
    setAmount('');
    setReason('');
    setRequestId(crypto.randomUUID());
    setKind('in');
    op.setError(null);
  };
  return (
    <div className="page-scroll page-container">
      <PageTitle
        eyebrow="EFECTIVO Y JORNADA"
        title="Cada peso, en su lugar."
        description="Consulta los movimientos y cierra tu caja con claridad."
        actions={
          data?.open ? (
            <>
              <Button variant="outline" onClick={() => open('move')}>
                <Plus size={16} />
                Ingreso / egreso
              </Button>
              <Button onClick={() => open('close')}>
                <LockKeyhole size={16} />
                Cerrar caja
              </Button>
            </>
          ) : (
            <Button disabled={loading || !data} onClick={() => open('open')}>
              <Wallet size={17} />
              Abrir caja
            </Button>
          )
        }
      />
      <div className="records-toolbar">
        <Button variant="outline" disabled={loading} onClick={refresh}>
          <RefreshCw size={16} />
          {loading ? 'Actualizando caja…' : 'Actualizar caja'}
        </Button>
      </div>
      <Failure error={loadError} retry={refresh} />
      <div className="cash-summary-grid" aria-busy={loading}>
        <div className="cash-main-card">
          <span>EFECTIVO ESPERADO</span>
          <h2>{data ? money(data.expectedCents) : '—'}</h2>
          <p>
            {data?.open ? `${data.open.folio} · Abierta ${data.open.openedAt}` : 'Sin caja abierta'}
          </p>
          <Wallet size={76} strokeWidth={1} />
        </div>
        <div className="metric-card">
          <span>Fondo inicial</span>
          <strong>{data ? money(data.open?.openingCents) : '—'}</strong>
          <small>Con lo que comenzó la jornada</small>
        </div>
        <div className="metric-card">
          <span>Movimientos</span>
          <strong>{data?.movementCount ?? 0}</strong>
          <small>Ingresos, ventas y egresos registrados</small>
        </div>
      </div>
      <section className="surface">
        <div className="section-heading">
          <h2>Movimientos de esta caja</h2>
          <span>{data?.open ? 'Jornada en curso' : 'Sin jornada activa'}</span>
        </div>
        {data?.movements.length ? (
          <DataGrid
            rows={data.movements}
            caption="Movimientos de la caja abierta"
            columns={[
              {
                id: 'date',
                label: 'Fecha',
                value: (row) => row.createdAt,
                render: (row) => dateTime(row.createdAt),
              },
              {
                id: 'kind',
                label: 'Movimiento',
                value: (row) => row.kind,
                render: (row) => (
                  <span
                    className={`movement-label ${row.amountCents < 0 ? 'negative' : 'positive'}`}
                  >
                    {row.amountCents < 0 ? <ArrowUpRight size={14} /> : <ArrowDownLeft size={14} />}{' '}
                    {row.kind}
                  </span>
                ),
              },
              { id: 'reference', label: 'Referencia', value: (row) => row.reference },
              { id: 'reason', label: 'Motivo', value: (row) => row.reason },
              {
                id: 'amount',
                label: 'Importe',
                value: (row) => row.amountCents,
                render: (row) => money(row.amountCents),
                align: 'right',
              },
            ]}
          />
        ) : (
          <Empty title={data?.open ? 'Tu caja está lista.' : 'Comienza abriendo la caja.'}>
            {data?.open
              ? 'Los cobros y movimientos aparecerán aquí.'
              : 'Indica el efectivo inicial para empezar a registrar ventas.'}
          </Empty>
        )}
      </section>
      {(data?.movementCount ?? 0) > 1000 && (
        <p className="muted">
          Se muestran los últimos 1,000 movimientos. El efectivo esperado incluye toda la jornada.
        </p>
      )}
      <section className="surface">
        <div className="section-heading">
          <h2>Cierres anteriores</h2>
        </div>
        <DataGrid
          rows={data?.history ?? []}
          caption="Últimos 200 cierres de caja"
          columns={[
            { id: 'folio', label: 'Folio', value: (row) => row.folio },
            {
              id: 'date',
              label: 'Cierre',
              value: (row) => row.closedAt,
              render: (row) => dateTime(row.closedAt ?? ''),
            },
            {
              id: 'expected',
              label: 'Esperado',
              value: (row) => row.expectedCents,
              render: (row) => money(row.expectedCents ?? 0),
              align: 'right',
            },
            {
              id: 'counted',
              label: 'Contado',
              value: (row) => row.countedCents,
              render: (row) => money(row.countedCents ?? 0),
              align: 'right',
            },
            {
              id: 'difference',
              label: 'Diferencia',
              value: (row) => (row.countedCents ?? 0) - (row.expectedCents ?? 0),
              render: (row) => money((row.countedCents ?? 0) - (row.expectedCents ?? 0)),
              align: 'right',
            },
          ]}
        />
      </section>
      <Modal
        open={!!modal}
        onClose={() => {
          if (!op.busy) setModal(null);
        }}
        title={
          modal === 'open'
            ? 'Abre tu caja'
            : modal === 'close'
              ? 'Cierre de jornada'
              : 'Registrar movimiento'
        }
        description={
          modal === 'close'
            ? 'Cuenta el efectivo físico y registra el resultado. El sistema conserva cualquier diferencia.'
            : modal === 'move'
              ? 'Elige si recibes o retiras efectivo y registra el motivo.'
              : 'Indica cuánto efectivo hay antes de comenzar.'
        }
      >
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void op.run(
              () =>
                modal === 'open'
                  ? api('cashOpen', { requestId, openingCents: parseScaled(amount) })
                  : modal === 'close'
                    ? api('cashClose', {
                        requestId,
                        countedCents: parseScaled(amount),
                        notes: reason,
                      })
                    : api('cashMovement', {
                        requestId,
                        amountCents: parseScaled(amount) * (kind === 'out' ? -1 : 1),
                        reason,
                      }),
              (v) => {
                setData(v);
                setModal(null);
              },
            );
          }}
        >
          {modal === 'move' && (
            <div className="dialog-actions" role="group" aria-label="Tipo de movimiento">
              <Button
                type="button"
                disabled={op.busy}
                variant={kind === 'in' ? 'default' : 'outline'}
                aria-pressed={kind === 'in'}
                onClick={() => setKind('in')}
              >
                <ArrowDownLeft size={17} />
                Ingreso
              </Button>
              <Button
                type="button"
                disabled={op.busy}
                variant={kind === 'out' ? 'default' : 'outline'}
                aria-pressed={kind === 'out'}
                onClick={() => setKind('out')}
              >
                <ArrowUpRight size={17} />
                Egreso
              </Button>
            </div>
          )}
          <Field
            label={
              modal === 'close'
                ? 'Efectivo contado'
                : modal === 'open'
                  ? 'Fondo inicial'
                  : kind === 'in'
                    ? 'Importe que recibes'
                    : 'Importe que retiras'
            }
          >
            <input
              autoFocus
              type="number"
              step="0.01"
              required
              min={modal === 'move' ? 0.01 : 0}
              max="1000000000"
              disabled={op.busy}
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
            />
          </Field>
          {modal !== 'open' && (
            <Field label={modal === 'move' ? 'Motivo' : 'Notas del cierre'}>
              <textarea
                disabled={op.busy}
                value={reason}
                onChange={(e) => setReason(e.target.value)}
                required={modal === 'move'}
                maxLength={300}
              />
            </Field>
          )}
          {modal === 'close' && (
            <div className="change-banner">
              <span>Diferencia</span>
              <b>{money(parseScaled(amount) - (data?.expectedCents ?? 0))}</b>
            </div>
          )}
          <Failure error={op.error} />
          <Button className="wide" disabled={op.busy}>
            {op.busy ? 'Guardando…' : 'Confirmar'}
            <Check size={16} />
          </Button>
        </form>
      </Modal>
    </div>
  );
}
