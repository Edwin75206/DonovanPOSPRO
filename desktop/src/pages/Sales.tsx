import { useState } from 'react';
import type { SortingState } from '@tanstack/react-table';
import { Search, ArrowRight, RotateCcw, ReceiptText } from 'lucide-react';
import { toast } from 'sonner';
import { api, type Bootstrap, type SaleDetail } from '@/lib/api';
import { useResource } from '@/lib/useResource';
import { dateTime, money, quantity, units, today, parseScaled } from '@/lib/utils';
import { Button, Busy, Failure, Field, Modal, PageTitle, useOperation } from '@/components/common';
import { DataGrid } from '@/components/DataGrid';
import { DocumentDialog } from '@/components/DocumentViewer';

export function Sales({ data, filters }: { data: Bootstrap; filters: Record<string, unknown> }) {
  const [from, setFrom] = useState(
    typeof filters.from === 'string' ? filters.from : `${today().slice(0, 7)}-01`,
  );
  const [to, setTo] = useState(typeof filters.to === 'string' ? filters.to : today());
  const [search, setSearch] = useState('');
  const [method, setMethod] = useState('all');
  const [status, setStatus] = useState('all');
  const [page, setPage] = useState(0);
  const [sorting, setSorting] = useState<SortingState>([{ id: 'folio', desc: true }]);
  const [selected, setSelected] = useState(
    typeof filters.saleId === 'number' ? filters.saleId : null,
  );
  const list = useResource(
    () =>
      api('salesPage', {
        from,
        to,
        search,
        method: method === 'all' ? null : Number(method),
        status,
        page,
        sort: sorting[0]?.id ?? 'folio',
        descending: sorting[0]?.desc ?? true,
      }),
    [from, to, search, method, status, page, sorting],
  );
  return (
    <main className="page-scroll page-container records-page">
      <PageTitle
        eyebrow="CADA OPERACIÓN EN SU LUGAR"
        title="Tu historial, siempre claro."
        description={
          data.actor?.role === 0
            ? 'Consulta ventas, pagos y devoluciones. Cada ticket conserva su historia.'
            : 'Consulta tus ventas y vuelve a imprimir cualquier ticket.'
        }
      />
      <div className="records-toolbar">
        <div className="records-search">
          <Search size={18} />
          <input
            aria-label="Buscar venta por folio, cliente o cajero"
            placeholder="Folio, cliente o cajero…"
            value={search}
            maxLength={300}
            onChange={(e) => {
              setSearch(e.target.value);
              setPage(0);
            }}
          />
        </div>
        <Field label="Venta desde">
          <input
            type="date"
            value={from}
            onChange={(e) => {
              setFrom(e.target.value);
              setPage(0);
            }}
          />
        </Field>
        <Field label="Hasta">
          <input
            type="date"
            value={to}
            onChange={(e) => {
              setTo(e.target.value);
              setPage(0);
            }}
          />
        </Field>
        <select
          aria-label="Filtrar forma de pago"
          value={method}
          onChange={(e) => {
            setMethod(e.target.value);
            setPage(0);
          }}
        >
          <option value="all">Todas las formas de pago</option>
          {['Efectivo', 'Tarjeta', 'Transferencia', 'Crédito'].map((m, i) => (
            <option key={m} value={i}>
              {m}
            </option>
          ))}
        </select>
        <select
          aria-label="Estado de la venta"
          value={status}
          onChange={(e) => {
            setStatus(e.target.value);
            setPage(0);
          }}
        >
          <option value="all">Todas las ventas</option>
          <option value="active">Sin cancelar</option>
          <option value="refunded">Con devoluciones</option>
        </select>
      </div>
      <Failure error={list.error} retry={list.reload} />
      <div className="records-metrics">
        <article>
          <span>Ventas encontradas</span>
          <strong>{list.data?.total ?? '—'}</strong>
        </article>
        <article>
          <span>Importe original</span>
          <strong>{list.data ? money(list.data.totalCents) : '—'}</strong>
        </article>
        <article>
          <span>Devoluciones de esas ventas a la fecha</span>
          <strong>{list.data ? money(list.data.refundedCents) : '—'}</strong>
        </article>
      </div>
      <section className="surface" aria-busy={list.busy}>
        <DataGrid
          rows={list.data?.items ?? []}
          caption="Historial de ventas"
          emptyTitle={list.busy ? 'Buscando ventas…' : 'No hay ventas con estos filtros.'}
          server={{
            page,
            pageSize: 60,
            total: list.data?.total ?? 0,
            onPage: setPage,
            sorting,
            onSorting: (next) => {
              setSorting(next);
              setPage(0);
            },
          }}
          columns={[
            {
              id: 'folio',
              label: 'Venta',
              value: (s) => s.folio,
              render: (s) => (
                <div className="record-name">
                  <span className="record-avatar">
                    <ReceiptText size={18} />
                  </span>
                  <div>
                    <strong>{s.folio}</strong>
                    <small>{dateTime(s.createdAt)}</small>
                  </div>
                </div>
              ),
            },
            {
              id: 'customer',
              label: 'Cliente / cajero',
              value: (s) => s.customerName,
              render: (s) => (
                <div className="two-line">
                  <strong>{s.customerName}</strong>
                  <small>{s.cashier}</small>
                </div>
              ),
            },
            {
              id: 'amount',
              label: 'Importe original',
              value: (s) => s.totalCents,
              render: (s) => <b>{money(s.totalCents)}</b>,
              align: 'right',
            },
            {
              id: 'refund',
              label: 'Devuelto',
              value: (s) => s.refundedCents,
              render: (s) => (s.refundedCents ? money(s.refundedCents) : '—'),
              align: 'right',
            },
            {
              id: 'state',
              label: 'Estado',
              value: (s) =>
                s.cancelled
                  ? 'Cancelada / devuelta'
                  : s.refundedCents
                    ? 'Devolución parcial'
                    : 'Registrada',
              render: (s) => (
                <span className={`record-state ${s.cancelled ? 'expired' : 'valid'}`}>
                  {s.cancelled
                    ? 'Cancelada / devuelta'
                    : s.refundedCents
                      ? 'Devolución parcial'
                      : 'Registrada'}
                </span>
              ),
            },
            {
              id: 'actions',
              label: 'Detalle',
              value: (s) => s.id,
              sortable: false,
              render: (s) => (
                <Button variant="outline" size="sm" onClick={() => setSelected(s.id)}>
                  Ver venta <ArrowRight size={15} />
                </Button>
              ),
            },
          ]}
        />
      </section>
      {selected && (
        <SaleViewer
          key={selected}
          id={selected}
          data={data}
          onClose={() => {
            setSelected(null);
            list.reload();
          }}
        />
      )}
    </main>
  );
}
export function SaleViewer({
  id,
  data,
  onClose,
}: {
  id: number;
  data: Bootstrap;
  onClose: () => void;
}) {
  const detail = useResource(() => api('sale', { id }), [id]);
  const [receipt, setReceipt] = useState(false);
  const [returning, setReturning] = useState(false);
  return (
    <Modal
      open
      onClose={onClose}
      title={detail.data ? `Venta ${detail.data.sale.folio}` : 'Detalle de venta'}
      description={
        detail.data
          ? `${detail.data.sale.customerName} · ${dateTime(detail.data.sale.createdAt)} · Atendió ${detail.data.sale.cashier}`
          : 'Recuperando el ticket…'
      }
      className="records-dialog"
    >
      <Failure error={detail.error} retry={detail.reload} />
      {!detail.data ? (
        <Busy />
      ) : (
        <>
          <DataGrid
            rows={detail.data.lines}
            caption="Productos vendidos"
            columns={[
              {
                id: 'name',
                label: 'Producto',
                value: (l) => l.name,
                render: (l) => (
                  <div className="two-line">
                    <strong>{l.name}</strong>
                    <small>{l.sku}</small>
                  </div>
                ),
              },
              {
                id: 'quantity',
                label: 'Cantidad',
                value: (l) => l.quantityUnits,
                render: (l) => `${quantity(l.quantityUnits)} ${units[l.unit]}`,
                align: 'right',
              },
              {
                id: 'price',
                label: 'Precio',
                value: (l) => l.priceCents,
                render: (l) => money(l.priceCents),
                align: 'right',
              },
              {
                id: 'total',
                label: 'Importe',
                value: (l) => l.totalCents,
                render: (l) => money(l.totalCents),
                align: 'right',
              },
              {
                id: 'returned',
                label: 'Devuelto',
                value: (l) => l.returnedUnits,
                render: (l) =>
                  l.returnedUnits ? `${quantity(l.returnedUnits)} ${units[l.unit]}` : '—',
                align: 'right',
              },
            ]}
          />
          <div className="account-summary">
            <div>
              <span>TOTAL ORIGINAL · IVA INCLUIDO</span>
              <strong>{money(detail.data.sale.totalCents)}</strong>
              <small>
                IVA: {money(detail.data.sale.taxCents)} · Descuento:{' '}
                {money(detail.data.sale.discountCents)}
              </small>
            </div>
            <div className="sale-payments">
              {detail.data.payments.map((p) => (
                <p key={p.method}>
                  <span>{['Efectivo', 'Tarjeta', 'Transferencia', 'Crédito'][p.method]}</span>
                  <b>{money(p.amountCents)}</b>
                </p>
              ))}
              <p>
                <span>Cambio</span>
                <b>{money(detail.data.sale.changeCents)}</b>
              </p>
              {detail.data.creditBalanceCents > 0 && (
                <p>
                  <span>Crédito pendiente</span>
                  <b>{money(detail.data.creditBalanceCents)}</b>
                </p>
              )}
            </div>
          </div>
          {!!detail.data.returns.length && (
            <>
              <h3>Devoluciones y cancelaciones</h3>
              <DataGrid
                rows={detail.data.returns}
                caption="Devoluciones de la venta"
                columns={[
                  { id: 'folio', label: 'Folio', value: (r) => r.folio },
                  {
                    id: 'date',
                    label: 'Fecha',
                    value: (r) => r.createdAt,
                    render: (r) => dateTime(r.createdAt),
                  },
                  { id: 'reason', label: 'Motivo', value: (r) => r.reason },
                  {
                    id: 'total',
                    label: 'Importe',
                    value: (r) => r.totalCents,
                    render: (r) => money(r.totalCents),
                    align: 'right',
                  },
                  {
                    id: 'credit',
                    label: 'Reducción de crédito',
                    value: (r) => r.creditReductionCents,
                    render: (r) => money(r.creditReductionCents),
                    align: 'right',
                  },
                ]}
              />
            </>
          )}
          <div className="dialog-actions">
            {data.actor?.role === 0 && !detail.data.sale.cancelled && (
              <Button variant="outline" onClick={() => setReturning(true)}>
                <RotateCcw size={17} /> Devolver / cancelar
              </Button>
            )}
            <Button onClick={() => setReceipt(true)}>
              <ReceiptText size={17} /> Ver y reimprimir ticket
            </Button>
          </div>
          {returning && (
            <ReturnForm
              detail={detail.data}
              onClose={() => setReturning(false)}
              onSaved={() => {
                setReturning(false);
                detail.reload();
              }}
            />
          )}
          {receipt && (
            <DocumentDialog
              data={data}
              request={{ kind: 'sale', id }}
              onClose={() => setReceipt(false)}
            />
          )}
        </>
      )}
    </Modal>
  );
}
function ReturnForm({
  detail,
  onClose,
  onSaved,
}: {
  detail: SaleDetail;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [cancel, setCancel] = useState(false);
  const [quantities, setQuantities] = useState<Record<number, string>>({});
  const [reason, setReason] = useState('');
  const [method, setMethod] = useState(0);
  const [confirmed, setConfirmed] = useState(false);
  const [requestId] = useState(() => crypto.randomUUID());
  const op = useOperation();
  const remaining = detail.lines.filter((l) => l.quantityUnits > l.returnedUnits);
  const items = remaining
    .map((l) => ({
      saleLineId: l.id,
      quantityUnits: cancel
        ? l.quantityUnits - l.returnedUnits
        : parseScaled(quantities[l.id] ?? '', 10000),
    }))
    .filter((i) => i.quantityUnits > 0);
  const valid =
    items.length > 0 &&
    items.every((i) => {
      const l = remaining.find((l) => l.id === i.saleLineId)!;
      return (
        i.quantityUnits <= l.quantityUnits - l.returnedUnits &&
        (![0, 4, 5].includes(l.unit) || i.quantityUnits % 10000 === 0)
      );
    });
  const total = valid
    ? items.reduce((sum, item) => {
        const l = remaining.find((l) => l.id === item.saleLineId)!;
        const denominator = BigInt(l.quantityUnits);
        const accumulated =
          (BigInt(l.totalCents) * BigInt(l.returnedUnits + item.quantityUnits) + denominator / 2n) /
          denominator;
        return sum + Number(accumulated) - l.refundedCents;
      }, 0)
    : 0;
  const creditReduction = Math.min(detail.creditBalanceCents, total);
  const refund = total - creditReduction;
  return (
    <Modal
      open
      onClose={() => {
        if (!op.busy) onClose();
      }}
      title={`Revisar devolución · ${detail.sale.folio}`}
      description="Las existencias se recuperan y primero se reduce el crédito pendiente de esta venta."
      className="records-dialog"
    >
      <form
        className="record-form"
        onSubmit={(e) => {
          e.preventDefault();
          if (!valid || !confirmed) return;
          void op.run(
            () =>
              api('returnSale', {
                requestId,
                saleId: detail.sale.id,
                items,
                cancel,
                reason,
                method,
                expectedTotalCents: total,
                expectedPaidRefundCents: refund,
              }),
            (result) => {
              toast.success(`${result.return.folio} · Devolución registrada`, {
                description:
                  result.paidRefundCents > 0
                    ? `Reembolso: ${money(result.paidRefundCents)} por ${['efectivo', 'tarjeta', 'transferencia'][method]}.`
                    : 'Se aplicó la reducción del crédito pendiente.',
              });
              onSaved();
            },
          );
        }}
      >
        <fieldset disabled={op.busy}>
          <label className="check-label">
            <input
              type="checkbox"
              checked={cancel}
              onChange={(e) => {
                setCancel(e.target.checked);
                setConfirmed(false);
              }}
            />{' '}
            Cancelar la venta y devolver todo lo pendiente
          </label>
          <DataGrid
            rows={remaining}
            caption="Seleccionar cantidades devueltas"
            columns={[
              { id: 'name', label: 'Producto', value: (l) => l.name },
              {
                id: 'available',
                label: 'Por devolver',
                value: (l) => l.quantityUnits - l.returnedUnits,
                render: (l) => `${quantity(l.quantityUnits - l.returnedUnits)} ${units[l.unit]}`,
              },
              {
                id: 'quantity',
                label: 'Devolver ahora',
                value: (l) => l.id,
                sortable: false,
                render: (l) => (
                  <input
                    className="table-number"
                    type="number"
                    min="0"
                    max={(l.quantityUnits - l.returnedUnits) / 10000}
                    step={[0, 4, 5].includes(l.unit) ? '1' : '0.0001'}
                    aria-label={`Devolver ${l.name}`}
                    disabled={cancel || op.busy}
                    value={
                      cancel
                        ? (l.quantityUnits - l.returnedUnits) / 10000
                        : (quantities[l.id] ?? '')
                    }
                    placeholder="0"
                    onChange={(e) => {
                      setQuantities((q) => ({ ...q, [l.id]: e.target.value }));
                      setConfirmed(false);
                    }}
                  />
                ),
              },
            ]}
          />
          <Field label="Motivo">
            <input
              required
              minLength={3}
              maxLength={300}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              placeholder="Describe por qué se devuelve o cancela…"
            />
          </Field>
          <div className="records-metrics">
            <article>
              <span>Importe de la devolución</span>
              <strong>{money(total)}</strong>
            </article>
            <article>
              <span>Reducción de crédito</span>
              <strong>{money(creditReduction)}</strong>
            </article>
            <article>
              <span>Dinero a reembolsar</span>
              <strong>{money(refund)}</strong>
            </article>
          </div>
          {refund > 0 && (
            <Field label="Forma de reembolso">
              <select
                value={method}
                onChange={(e) => {
                  setMethod(Number(e.target.value));
                  setConfirmed(false);
                }}
              >
                {['Efectivo', 'Tarjeta', 'Transferencia'].map((m, i) => (
                  <option key={m} value={i}>
                    {m}
                  </option>
                ))}
              </select>
            </Field>
          )}
          <label className="check-label">
            <input
              type="checkbox"
              required
              checked={confirmed}
              onChange={(e) => setConfirmed(e.target.checked)}
            />{' '}
            Revisé las cantidades y confirmo {cancel ? 'la cancelación' : 'la devolución'} de{' '}
            {money(total)}
            {refund > 0 ? `, con reembolso de ${money(refund)}` : ''}.
          </label>
          <p className="muted">
            Si el saldo o la venta cambia durante esta revisión, no se aplicará la operación. El
            reembolso en efectivo requiere una caja abierta con fondos suficientes.
          </p>
        </fieldset>
        <Failure error={op.error} />
        <div className="dialog-actions">
          <Button type="button" variant="outline" disabled={op.busy} onClick={onClose}>
            Volver sin cambios
          </Button>
          <Button type="submit" variant="destructive" disabled={op.busy || !valid || !confirmed}>
            {op.busy ? 'Registrando…' : cancel ? 'Confirmar cancelación' : 'Confirmar devolución'}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
