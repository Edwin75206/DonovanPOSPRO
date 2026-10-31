import { useEffect, useState } from 'react';
import { Wallet, ArrowDownLeft, ArrowUpRight, Check, Plus, LockKeyhole } from 'lucide-react';
import { api, type CashOverview } from '@/lib/api';
import {
  Button,
  PageTitle,
  Field,
  Modal,
  Failure,
  useOperation,
  reportError,
  Empty,
} from '@/components/common';
import { money, parseScaled } from '@/lib/utils';
export function Cash() {
  const [data, setData] = useState<CashOverview | null>(null);
  const [modal, setModal] = useState<'open' | 'move' | 'close' | null>(null);
  const [amount, setAmount] = useState('');
  const [reason, setReason] = useState('');
  const op = useOperation();
  useEffect(() => {
    void api('cashOverview').then(setData).catch(reportError);
  }, []);
  const open = (action: typeof modal) => {
    setModal(action);
    setAmount('');
    setReason('');
    op.setError(null);
  };
  return (
    <div className="page-scroll">
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
            <Button onClick={() => open('open')}>
              <Wallet size={17} />
              Abrir caja
            </Button>
          )
        }
      />
      <div className="cash-summary-grid">
        <div className="cash-main-card">
          <span>EFECTIVO ESPERADO</span>
          <h2>{money(data?.expectedCents)}</h2>
          <p>
            {data?.open ? `${data.open.folio} · Abierta ${data.open.openedAt}` : 'Sin caja abierta'}
          </p>
          <Wallet size={76} strokeWidth={1} />
        </div>
        <div className="metric-card">
          <span>Fondo inicial</span>
          <strong>{money(data?.open?.openingCents)}</strong>
          <small>Con lo que comenzó la jornada</small>
        </div>
        <div className="metric-card">
          <span>Movimientos</span>
          <strong>{data?.movements.length ?? 0}</strong>
          <small>Ingresos, ventas y egresos registrados</small>
        </div>
      </div>
      <section className="surface">
        <div className="section-heading">
          <h2>Movimientos de esta caja</h2>
          <span>{data?.open ? 'Jornada en curso' : 'Sin jornada activa'}</span>
        </div>
        {data?.movements.length ? (
          <table className="data-table">
            <thead>
              <tr>
                <th>Hora</th>
                <th>Movimiento</th>
                <th>Referencia</th>
                <th>Motivo</th>
                <th className="numeric">Importe</th>
              </tr>
            </thead>
            <tbody>
              {data.movements.map((row) => (
                <tr key={row.id}>
                  <td>{row.createdAt}</td>
                  <td>
                    <span
                      className={`movement-label ${row.amountCents < 0 ? 'negative' : 'positive'}`}
                    >
                      {row.amountCents < 0 ? (
                        <ArrowUpRight size={14} />
                      ) : (
                        <ArrowDownLeft size={14} />
                      )}{' '}
                      {row.kind}
                    </span>
                  </td>
                  <td>{row.reference}</td>
                  <td>{row.reason || '—'}</td>
                  <td className="numeric">{money(row.amountCents)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <Empty title={data?.open ? 'Tu caja está lista.' : 'Comienza abriendo la caja.'}>
            {data?.open
              ? 'Los cobros y movimientos aparecerán aquí.'
              : 'Indica el efectivo inicial para empezar a registrar ventas.'}
          </Empty>
        )}
      </section>
      <section className="surface">
        <div className="section-heading">
          <h2>Cierres anteriores</h2>
        </div>
        <table className="data-table">
          <thead>
            <tr>
              <th>Folio</th>
              <th>Cierre</th>
              <th className="numeric">Esperado</th>
              <th className="numeric">Contado</th>
              <th className="numeric">Diferencia</th>
            </tr>
          </thead>
          <tbody>
            {data?.history.map((row) => (
              <tr key={row.id}>
                <td>{row.folio}</td>
                <td>{row.closedAt}</td>
                <td className="numeric">{money(row.expectedCents ?? 0)}</td>
                <td className="numeric">{money(row.countedCents ?? 0)}</td>
                <td className="numeric">
                  {money((row.countedCents ?? 0) - (row.expectedCents ?? 0))}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
      <Modal
        open={!!modal}
        onClose={() => setModal(null)}
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
              ? 'Un importe positivo registra una entrada; uno negativo, una salida de efectivo.'
              : 'Indica cuánto efectivo hay antes de comenzar.'
        }
      >
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void op.run(
              () =>
                modal === 'open'
                  ? api('cashOpen', { openingCents: parseScaled(amount) })
                  : modal === 'close'
                    ? api('cashClose', { countedCents: parseScaled(amount), notes: reason })
                    : api('cashMovement', { amountCents: parseScaled(amount), reason }),
              (v) => {
                setData(v);
                setModal(null);
              },
            );
          }}
        >
          <Field
            label={
              modal === 'close'
                ? 'Efectivo contado'
                : modal === 'open'
                  ? 'Fondo inicial'
                  : 'Importe (+ entrada / − salida)'
            }
          >
            <input
              autoFocus
              type="number"
              step="0.01"
              required
              min={modal === 'move' ? undefined : 0}
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
            />
          </Field>
          {modal !== 'open' && (
            <Field label={modal === 'move' ? 'Motivo' : 'Notas del cierre'}>
              <textarea
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
