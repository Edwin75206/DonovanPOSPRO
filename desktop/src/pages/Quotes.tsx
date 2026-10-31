import { useState } from 'react';
import { Search, ArrowRight, Plus, FileText, Printer, PencilLine } from 'lucide-react';
import { api, type Quote, type QuoteDetail, type Product, type Bootstrap } from '@/lib/api';
import { DocumentDialog } from '@/components/DocumentViewer';
import { useResource } from '@/lib/useResource';
import { dateTime, money, quantity, units, today } from '@/lib/utils';
import { Button, Busy, Failure, Modal, PageTitle, useOperation } from '@/components/common';
import { DataGrid } from '@/components/DataGrid';
import { freshCart, type Cart } from './Register';

export function Quotes({
  data,
  hasCart,
  onSale,
  onLoad,
}: {
  data: Bootstrap;
  hasCart: boolean;
  onSale: () => void;
  onLoad: (cart: Cart) => void;
}) {
  const [search, setSearch] = useState('');
  const [status, setStatus] = useState('all');
  const [selected, setSelected] = useState<number | null>(null);
  const list = useResource(() => api('quotes', { search }), [search]);
  const state = (quote: Quote) =>
    quote.saleId ? 'sold' : quote.validUntil < today() ? 'expired' : 'valid';
  const stateName = { sold: 'Convertida en venta', expired: 'Vencida', valid: 'Vigente' };
  return (
    <main className="page-scroll page-container records-page">
      <PageTitle
        eyebrow="DE LA IDEA A LA VENTA"
        title="Propuestas que se convierten."
        description="Precios acordados, vigencias claras y un paso directo al cobro."
        actions={
          <Button onClick={onSale}>
            <Plus size={17} /> Crear desde el punto de venta
          </Button>
        }
      />
      <div className="records-toolbar">
        <div className="records-search">
          <Search size={18} />
          <input
            placeholder="Busca por folio o cliente…"
            aria-label="Buscar cotización"
            value={search}
            maxLength={300}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        <select
          aria-label="Estado de cotización"
          value={status}
          onChange={(e) => setStatus(e.target.value)}
        >
          <option value="all">Todas las cotizaciones</option>
          <option value="valid">Vigentes</option>
          <option value="expired">Vencidas</option>
          <option value="sold">Convertidas en venta</option>
        </select>
        <span>{list.busy ? 'Buscando…' : `${list.data?.length ?? 0} resultados`}</span>
      </div>
      <Failure error={list.error} retry={list.reload} />
      <section className="surface">
        <DataGrid
          rows={(list.data ?? []).filter((q) => status === 'all' || state(q) === status)}
          caption="Cotizaciones del negocio"
          emptyTitle={list.busy ? 'Buscando cotizaciones…' : 'Tu próxima venta puede empezar aquí.'}
          columns={[
            {
              id: 'folio',
              label: 'Cotización',
              value: (q) => q.folio,
              render: (q) => (
                <div className="record-name">
                  <span className="record-avatar">
                    <FileText size={18} />
                  </span>
                  <div>
                    <strong>{q.folio}</strong>
                    <small>{dateTime(q.createdAt)}</small>
                  </div>
                </div>
              ),
            },
            { id: 'customer', label: 'Cliente', value: (q) => q.customerName },
            { id: 'valid', label: 'Válida hasta', value: (q) => q.validUntil },
            {
              id: 'total',
              label: 'Importe',
              value: (q) => q.totalCents,
              render: (q) => <b>{money(q.totalCents)}</b>,
              align: 'right',
            },
            {
              id: 'status',
              label: 'Estado',
              value: (q) => stateName[state(q)],
              render: (q) => (
                <span className={`record-state ${state(q)}`}>{stateName[state(q)]}</span>
              ),
            },
            {
              id: 'actions',
              label: 'Detalle',
              value: (q) => q.id,
              sortable: false,
              render: (q) => (
                <Button variant="outline" size="sm" onClick={() => setSelected(q.id)}>
                  Ver cotización <ArrowRight size={15} />
                </Button>
              ),
            },
          ]}
        />
      </section>
      {list.data?.length === 1000 && (
        <p className="muted">
          Mostramos las últimas 1,000 coincidencias. Busca por folio o cliente para acotar el
          resultado.
        </p>
      )}
      {selected && (
        <QuoteViewer
          data={data}
          key={selected}
          id={selected}
          hasCart={hasCart}
          onSale={onSale}
          onClose={() => setSelected(null)}
          onLoad={onLoad}
        />
      )}
    </main>
  );
}
function QuoteViewer({
  data,
  id,
  hasCart,
  onClose,
  onSale,
  onLoad,
}: {
  data: Bootstrap;
  id: number;
  hasCart: boolean;
  onClose: () => void;
  onSale: () => void;
  onLoad: (cart: Cart) => void;
}) {
  const detail = useResource(() => api('quote', { id }), [id]);
  const op = useOperation();
  const quote = detail.data?.quote;
  const [paper, setPaper] = useState(false);
  const [editing, setEditing] = useState(false);
  return (
    <Modal
      open
      onClose={() => {
        if (!op.busy) onClose();
      }}
      title={quote ? `Cotización ${quote.folio}` : 'Cotización'}
      description={
        quote
          ? `${quote.customerName} · Válida hasta ${quote.validUntil}`
          : 'Consultando la propuesta…'
      }
      className="records-dialog"
    >
      <Failure error={detail.error ?? op.error} retry={detail.reload} />
      {!detail.data ? (
        <Busy />
      ) : (
        <>
          <QuoteLines detail={detail.data} />
          {quote?.notes && <p className="notice">{quote.notes}</p>}
          <Button variant="outline" disabled={op.busy} onClick={() => setPaper(true)}>
            <Printer size={17} /> Ver PDF e imprimir
          </Button>
          {paper && (
            <DocumentDialog
              data={data}
              request={{ kind: 'quote', id }}
              onClose={() => setPaper(false)}
            />
          )}
          <div className="account-summary">
            <div>
              <span>IMPORTE ACORDADO · IVA INCLUIDO</span>
              <strong>{money(quote?.totalCents)}</strong>
              <small>IVA: {money(quote?.taxCents)} · Se conservan los descuentos acordados.</small>
            </div>
          </div>
          {hasCart && !quote?.saleId && (
            <p className="notice">
              Tienes una venta en curso.{' '}
              <button className="text-button" onClick={onSale}>
                Vuelve al ticket para guardarla
              </button>{' '}
              antes de recuperar esta cotización.
            </p>
          )}
          {quote?.saleId ? (
            <p className="notice">
              Esta cotización ya se convirtió en venta. Su historial se conserva.
            </p>
          ) : quote && quote.validUntil < today() ? (
            <p className="notice">
              La cotización venció. Puedes editar sus partidas y renovar la vigencia con precios
              actuales.
            </p>
          ) : (
            <Button
              disabled={hasCart || op.busy}
              onClick={() =>
                void op.run(async () => {
                  const current = await api('quote', { id });
                  await api('price', { items: [], discountParts: 0, quoteId: id });
                  const rows = current.lines.map((line) => ({
                    product: {
                      id: line.productId,
                      sku: line.sku,
                      barcode: null,
                      name: line.name,
                      category: 'Cotización',
                      brand: 'Precio acordado',
                      size: '',
                      aliases: '',
                      unit: line.unit,
                      costCents: null,
                      priceCents: line.priceCents,
                      wholesaleCents: line.priceCents,
                      stockUnits: line.stockUnits,
                      minimumUnits: 0,
                      taxBasisPoints: line.taxBasisPoints,
                      packName: 'Caja',
                      packUnits: 10000,
                      active: line.active,
                    } satisfies Product,
                    quantityUnits: line.quantityUnits,
                    wholesale: false,
                  }));
                  return {
                    ...freshCart(),
                    rows,
                    customerId: current.quote.customerId,
                    customerName: current.quote.customerName,
                    quoteId: id,
                    sourceLabel: current.quote.folio,
                  };
                }, onLoad)
              }
            >
              {op.busy ? 'Preparando venta…' : 'Continuar al punto de venta'}{' '}
              <ArrowRight size={17} />
            </Button>
          )}
          {quote && !quote.saleId && (
            <Button
              variant="outline"
              disabled={hasCart || op.busy}
              onClick={() => setEditing(true)}
            >
              <PencilLine size={16} />
              Editar partidas y vigencia
            </Button>
          )}
          {editing && (
            <Modal
              open
              title={`Editar ${quote?.folio}`}
              description="Se usarán los precios actuales de menudeo y el descuento comenzará en cero. La propuesta anterior se conserva hasta que guardes los cambios."
              onClose={() => {
                if (!op.busy) setEditing(false);
              }}
            >
              <p>
                Podrás cambiar productos, cantidades, cliente, descuento y vigencia antes de
                guardar. Si ya venció, te propondremos una vigencia de 15 días.
              </p>
              <Failure error={op.error} />
              <div className="dialog-actions">
                <Button variant="outline" disabled={op.busy} onClick={() => setEditing(false)}>
                  Volver
                </Button>
                <Button
                  disabled={op.busy || hasCart}
                  onClick={() =>
                    void op.run(async () => {
                      const current = await api('quote', { id });
                      // Let the native service return its audited, actionable error.
                      if (current.quote.saleId)
                        await api('price', { items: [], discountParts: 0, quoteId: id });
                      const products = await Promise.all(
                        current.lines.map((line) => api('product', { id: line.productId })),
                      );
                      return {
                        ...freshCart(),
                        rows: current.lines.map((line, index) => ({
                          product: products[index],
                          quantityUnits: line.quantityUnits,
                          wholesale: false,
                        })),
                        customerId: current.quote.customerId,
                        customerName: current.quote.customerName,
                        sourceLabel: current.quote.folio,
                        editingQuote: {
                          id,
                          notes: current.quote.notes,
                          validUntil: current.quote.validUntil,
                        },
                      };
                    }, onLoad)
                  }
                >
                  {op.busy ? 'Preparando…' : 'Abrir para editar'}
                  <ArrowRight size={16} />
                </Button>
              </div>
            </Modal>
          )}
        </>
      )}
    </Modal>
  );
}
export function QuoteLines({ detail }: { detail: QuoteDetail }) {
  return (
    <DataGrid
      rows={detail.lines}
      caption="Partidas de la cotización"
      columns={[
        {
          id: 'name',
          label: 'Producto',
          value: (l) => l.name,
          render: (l) => (
            <div className="two-line">
              <strong>{l.name}</strong>
              <small>
                {l.sku}
                {!l.active
                  ? ' · Archivado'
                  : l.stockUnits < l.quantityUnits
                    ? ' · Existencias insuficientes'
                    : ''}
              </small>
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
          label: 'Precio antes de descuento',
          value: (l) => l.priceCents,
          render: (l) => money(l.priceCents),
          align: 'right',
        },
        {
          id: 'total',
          label: 'Importe acordado',
          value: (l) => l.totalCents,
          render: (l) => <b>{money(l.totalCents)}</b>,
          align: 'right',
        },
      ]}
    />
  );
}
