import { useState } from 'react';
import { Search, Plus, Truck, ArrowRight, Trash2, PackageCheck, Printer } from 'lucide-react';
import { toast } from 'sonner';
import { api, type Contact, type Product, type Bootstrap } from '@/lib/api';
import { DocumentDialog } from '@/components/DocumentViewer';
import { useResource } from '@/lib/useResource';
import { dateTime, money, quantity, units, today, parseScaled } from '@/lib/utils';
import { Button, Busy, Failure, Field, Modal, PageTitle, useOperation } from '@/components/common';
import { DataGrid } from '@/components/DataGrid';
import { ContactEditor, newContact } from './Contacts';

export function Purchases({ data, onSuppliers }: { data: Bootstrap; onSuppliers: () => void }) {
  const [search, setSearch] = useState('');
  const [create, setCreate] = useState(false);
  const [selected, setSelected] = useState<number | null>(null);
  const list = useResource(() => api('purchases', { search }), [search]);
  return (
    <main className="page-scroll page-container records-page">
      <PageTitle
        eyebrow="TU INVENTARIO EN MOVIMIENTO"
        title="Recibe hoy, vende mejor."
        description="Registra lo que llega, controla sus costos y conserva el comprobante del proveedor."
        actions={
          <>
            <Button variant="outline" onClick={onSuppliers}>
              <Truck size={17} /> Proveedores
            </Button>
            <Button onClick={() => setCreate(true)}>
              <Plus size={17} /> Recibir compra
            </Button>
          </>
        }
      />
      <div className="records-toolbar">
        <div className="records-search">
          <Search size={18} />
          <input
            value={search}
            aria-label="Buscar compra"
            placeholder="Folio, factura o proveedor…"
            maxLength={300}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        <span>{list.busy ? 'Buscando…' : `${list.data?.length ?? 0} compras`}</span>
      </div>
      <Failure error={list.error} retry={list.reload} />
      <section className="surface">
        <DataGrid
          rows={list.data ?? []}
          caption="Compras recibidas"
          emptyTitle={list.busy ? 'Buscando compras…' : 'La próxima entrega empieza aquí.'}
          columns={[
            {
              id: 'folio',
              label: 'Recepción',
              value: (p) => p.folio,
              render: (p) => (
                <div className="record-name">
                  <span className="record-avatar">
                    <PackageCheck size={18} />
                  </span>
                  <div>
                    <strong>{p.folio}</strong>
                    <small>{dateTime(p.createdAt)}</small>
                  </div>
                </div>
              ),
            },
            { id: 'supplier', label: 'Proveedor', value: (p) => p.supplierName },
            { id: 'reference', label: 'Folio del proveedor', value: (p) => p.supplierReference },
            {
              id: 'amount',
              label: 'Importe',
              value: (p) => p.totalCents,
              render: (p) => <b>{money(p.totalCents)}</b>,
              align: 'right',
            },
            {
              id: 'payment',
              label: 'Salida de caja',
              value: (p) =>
                p.paidFromCash === null ? 'Sin detalle histórico' : p.paidFromCash ? 'Sí' : 'No',
              sortable: false,
            },
            {
              id: 'actions',
              label: 'Detalle',
              value: (p) => p.id,
              sortable: false,
              render: (p) => (
                <Button size="sm" variant="outline" onClick={() => setSelected(p.id)}>
                  Ver compra <ArrowRight size={15} />
                </Button>
              ),
            },
          ]}
        />
      </section>
      {list.data?.length === 1000 && (
        <p className="muted">
          Mostramos las últimas 1,000 coincidencias. Acota la búsqueda por proveedor o folio.
        </p>
      )}
      {create && (
        <PurchaseForm
          onClose={() => setCreate(false)}
          onSaved={(id) => {
            setCreate(false);
            list.reload();
            setSelected(id);
          }}
        />
      )}
      {selected && <PurchaseViewer data={data} id={selected} onClose={() => setSelected(null)} />}
    </main>
  );
}
function PurchaseViewer({
  data,
  id,
  onClose,
}: {
  data: Bootstrap;
  id: number;
  onClose: () => void;
}) {
  const detail = useResource(() => api('purchase', { id }), [id]);
  const [paper, setPaper] = useState(false);
  return (
    <Modal
      open
      onClose={onClose}
      title={detail.data?.purchase.folio ?? 'Compra recibida'}
      description={
        detail.data
          ? `${detail.data.purchase.supplierName} · Folio ${detail.data.purchase.supplierReference} · ${dateTime(detail.data.purchase.createdAt)}`
          : 'Consultando la compra…'
      }
      className="records-dialog"
    >
      <Failure error={detail.error} retry={detail.reload} />
      {!detail.data ? (
        <Busy />
      ) : (
        <>
          <Button variant="outline" onClick={() => setPaper(true)}>
            <Printer size={17} /> Ver PDF e imprimir
          </Button>
          {paper && (
            <DocumentDialog
              data={data}
              request={{ kind: 'purchase', id }}
              onClose={() => setPaper(false)}
            />
          )}
          {!detail.data.purchase.sourceExact && (
            <p className="notice">
              Este registro histórico conserva el total original y las unidades recibidas. No
              contiene el desglose original por empaque.
            </p>
          )}
          <DataGrid
            rows={detail.data.lines}
            caption="Partidas de compra"
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
                id: 'entered',
                label: 'Recibido',
                value: (l) => l.enteredQuantityUnits ?? l.quantityUnits,
                render: (l) =>
                  l.enteredQuantityUnits !== null
                    ? `${quantity(l.enteredQuantityUnits)} ${l.byPack ? l.packName : units[l.unit]}`
                    : `${quantity(l.quantityUnits)} ${units[l.unit]}`,
                align: 'right',
              },
              {
                id: 'units',
                label: 'Entrada a inventario',
                value: (l) => l.quantityUnits,
                render: (l) => `${quantity(l.quantityUnits)} ${units[l.unit]}`,
                align: 'right',
              },
              {
                id: 'cost',
                label: 'Costo por presentación',
                value: (l) => l.enteredCostCents ?? l.costCents,
                render: (l) => money(l.enteredCostCents ?? l.costCents),
                align: 'right',
              },
              {
                id: 'total',
                label: 'Importe original',
                value: (l) => l.totalCents,
                render: (l) =>
                  l.totalCents === null ? 'Sin detalle original' : money(l.totalCents),
                align: 'right',
              },
            ]}
          />
          <div className="account-summary">
            <div>
              <span>TOTAL DE LA COMPRA</span>
              <strong>{money(detail.data.purchase.totalCents)}</strong>
              <small>
                {detail.data.purchase.paidFromCash === null
                  ? 'Sin detalle del pago en la versión original.'
                  : detail.data.purchase.paidFromCash
                    ? 'Pagada con efectivo de caja.'
                    : 'Sin retiro de efectivo de caja.'}
              </small>
            </div>
          </div>
        </>
      )}
    </Modal>
  );
}
type PurchaseRow = { id: number; product: Product; amount: string; cost: string; byPack: boolean };
function PurchaseForm({
  onClose,
  onSaved,
}: {
  onClose: () => void;
  onSaved: (id: number) => void;
}) {
  const [supplier, setSupplier] = useState<Contact | null>(null);
  const [pickSupplier, setPickSupplier] = useState(false);
  const [search, setSearch] = useState('');
  const [rows, setRows] = useState<PurchaseRow[]>([]);
  const [reference, setReference] = useState('');
  const [date, setDate] = useState(today);
  const [paidFromCash, setPaidFromCash] = useState(false);
  const [confirmed, setConfirmed] = useState(false);
  const [abandon, setAbandon] = useState(false);
  const [requestId] = useState(() => crypto.randomUUID());
  const products = useResource(
    () => api('products', { search, category: '', status: 'active', page: 0 }),
    [search],
  );
  const op = useOperation();
  const update = (id: number, partial: Partial<PurchaseRow>) => {
    setRows((all) => all.map((r) => (r.id === id ? { ...r, ...partial } : r)));
    setConfirmed(false);
  };
  const lineTotal = (row: PurchaseRow) =>
    Number(
      (BigInt(Math.max(0, parseScaled(row.cost))) *
        BigInt(Math.max(0, parseScaled(row.amount, 10000))) +
        5000n) /
        10000n,
    );
  const total = rows.reduce((sum, row) => sum + lineTotal(row), 0);
  const close = () => {
    if (!op.busy) {
      if (rows.length || reference) setAbandon(true);
      else onClose();
    }
  };
  return (
    <Modal
      open
      onClose={close}
      title="Recibir una compra"
      description="Captura el folio del proveedor y los productos que recibiste."
      className="records-dialog"
    >
      <form
        className="record-form"
        onSubmit={(e) => {
          e.preventDefault();
          if (!supplier || !confirmed || !rows.length) return;
          const invalid = rows.find((row) => {
            const amount = parseScaled(row.amount, 10000);
            const cost = parseScaled(row.cost);
            return (
              !/^\d+(?:\.\d{1,2})?$/.test(row.cost) ||
              cost > 100000000000 ||
              !/^\d+(?:\.\d{1,4})?$/.test(row.amount) ||
              amount <= 0 ||
              amount > 1000000000000 ||
              ((row.byPack || [0, 4, 5].includes(row.product.unit)) && amount % 10000 !== 0)
            );
          });
          if (invalid) {
            toast.error(
              `Revisa el costo y la cantidad de ${invalid.product.name} (${invalid.product.sku}). Todas las partidas deben estar completas.`,
            );
            return;
          }
          if (!Number.isSafeInteger(total) || total > 100000000000) {
            toast.error(
              'El total supera el importe permitido para una compra. Revisa las cantidades y costos.',
            );
            return;
          }
          void op.run(
            () =>
              api('receivePurchase', {
                requestId,
                supplierId: supplier.id,
                reference,
                date,
                paidFromCash,
                items: rows.map((r) => ({
                  productId: r.id,
                  quantityUnits: parseScaled(r.amount, 10000),
                  unitCostCents: parseScaled(r.cost),
                  byPack: r.byPack,
                })),
              }),
            (result) => {
              toast.success(
                `${result.folio} · Compra recibida. Las existencias y costos se actualizaron.`,
              );
              onSaved(result.id);
            },
          );
        }}
      >
        <fieldset disabled={op.busy}>
          <div className="purchase-header">
            <Field label="Proveedor">
              <Button type="button" variant="outline" onClick={() => setPickSupplier(true)}>
                <Truck size={17} />
                {supplier?.name ?? 'Seleccionar proveedor'}
              </Button>
            </Field>
            <Field label="Folio / factura del proveedor">
              <input
                required
                maxLength={100}
                value={reference}
                onChange={(e) => {
                  setReference(e.target.value);
                  setConfirmed(false);
                }}
              />
            </Field>
            <Field label="Fecha del comprobante">
              <input
                required
                type="date"
                max={today()}
                value={date}
                onChange={(e) => setDate(e.target.value)}
              />
            </Field>
          </div>
          <div className="records-search">
            <Search size={18} />
            <input
              aria-label="Agregar producto a compra"
              placeholder="Busca por código, nombre, marca o medida…"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              maxLength={300}
            />
          </div>
          {search && (
            <div className="purchase-suggestions">
              {products.data?.items.slice(0, 8).map((p) => (
                <button
                  key={p.id}
                  type="button"
                  disabled={rows.length >= 500}
                  onClick={() => {
                    setRows((current) =>
                      current.some((r) => r.id === p.id)
                        ? current.map((r) =>
                            r.id === p.id
                              ? {
                                  ...r,
                                  amount: String((parseScaled(r.amount, 10000) + 10000) / 10000),
                                }
                              : r,
                          )
                        : [
                            ...current,
                            { id: p.id, product: p, amount: '1', cost: '', byPack: false },
                          ],
                    );
                    setConfirmed(false);
                    setSearch('');
                  }}
                >
                  <span>
                    <strong>{p.name}</strong>
                    <small>
                      {p.sku} · {p.brand} · {p.size}
                    </small>
                  </span>
                  <Plus size={17} />
                </button>
              ))}
              {products.busy ? (
                <p>Buscando…</p>
              ) : (
                !products.data?.items.length && (
                  <p>No encontramos productos activos con esa búsqueda.</p>
                )
              )}
            </div>
          )}
          <Failure error={products.error} />
          <DataGrid
            rows={rows}
            caption="Productos de la compra"
            emptyTitle="Busca un producto para agregarlo a la recepción."
            columns={[
              {
                id: 'product',
                label: 'Producto',
                value: (r) => r.product.name,
                render: (r) => (
                  <div className="two-line">
                    <strong>{r.product.name}</strong>
                    <small>
                      {r.product.sku} · Costo unitario actual:{' '}
                      {r.product.costCents === null ? 'sin registrar' : money(r.product.costCents)}
                    </small>
                  </div>
                ),
              },
              {
                id: 'presentation',
                label: 'Presentación',
                value: (r) => (r.byPack ? r.product.packName : units[r.product.unit]),
                sortable: false,
                render: (r) => (
                  <div className="purchase-presentation">
                    <select
                      aria-label={`Presentación de ${r.product.name}`}
                      value={r.byPack ? 'pack' : 'unit'}
                      onChange={(e) =>
                        update(r.id, { byPack: e.target.value === 'pack', amount: '1', cost: '' })
                      }
                    >
                      <option value="unit">{units[r.product.unit]}</option>
                      <option value="pack">
                        {r.product.packName} · {quantity(r.product.packUnits)}{' '}
                        {units[r.product.unit]}
                      </option>
                    </select>
                    <small>
                      Entrarán{' '}
                      {quantity(
                        r.byPack
                          ? Number(
                              (BigInt(parseScaled(r.amount, 10000)) * BigInt(r.product.packUnits)) /
                                10000n,
                            )
                          : parseScaled(r.amount, 10000),
                      )}{' '}
                      {units[r.product.unit]}
                    </small>
                  </div>
                ),
              },
              {
                id: 'quantity',
                label: 'Cantidad',
                value: (r) => parseScaled(r.amount, 10000),
                sortable: false,
                render: (r) => (
                  <input
                    aria-label={`Cantidad recibida de ${r.product.name}`}
                    className="table-number"
                    type="number"
                    min={r.byPack || [0, 4, 5].includes(r.product.unit) ? 1 : 0.0001}
                    max="100000000"
                    step={r.byPack || [0, 4, 5].includes(r.product.unit) ? 1 : 0.0001}
                    required
                    value={r.amount}
                    onChange={(e) => update(r.id, { amount: e.target.value })}
                  />
                ),
              },
              {
                id: 'cost',
                label: 'Costo por presentación',
                value: (r) => parseScaled(r.cost),
                sortable: false,
                render: (r) => (
                  <input
                    aria-label={`Costo por ${r.byPack ? r.product.packName : units[r.product.unit]} de ${r.product.name}`}
                    className="table-number"
                    type="number"
                    min="0"
                    max="1000000000"
                    step=".01"
                    required
                    value={r.cost}
                    placeholder="0.00"
                    onChange={(e) => update(r.id, { cost: e.target.value })}
                  />
                ),
              },
              {
                id: 'total',
                label: 'Importe',
                value: lineTotal,
                render: (r) => money(lineTotal(r)),
                align: 'right',
              },
              {
                id: 'remove',
                label: 'Quitar',
                value: (r) => r.id,
                sortable: false,
                render: (r) => (
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    aria-label={`Quitar ${r.product.name}`}
                    onClick={() => {
                      setRows((all) => all.filter((x) => x.id !== r.id));
                      setConfirmed(false);
                    }}
                  >
                    <Trash2 size={15} />
                  </Button>
                ),
              },
            ]}
          />
          <div className="account-summary">
            <div>
              <span>TOTAL DE LA COMPRA</span>
              <strong>{money(total)}</strong>
              <small>
                {rows.length} partidas · Se conservará el importe exacto de cada presentación.
              </small>
            </div>
            <label className="check-label">
              <input
                type="checkbox"
                checked={paidFromCash}
                onChange={(e) => {
                  setPaidFromCash(e.target.checked);
                  setConfirmed(false);
                }}
              />{' '}
              Pagar desde la caja abierta
            </label>
          </div>
          <label className="check-label">
            <input
              type="checkbox"
              required
              checked={confirmed}
              onChange={(e) => setConfirmed(e.target.checked)}
            />{' '}
            Recibí estos productos y revisé el folio, las cantidades y el total de {money(total)}.
          </label>
          <p className="muted">
            Los precios de venta se conservan. Se actualizan las existencias y el costo promedio;
            cuando no había un costo registrado, se toma el costo de esta compra.
          </p>
        </fieldset>
        <Failure error={op.error} />
        <div className="dialog-actions">
          <Button type="button" variant="outline" disabled={op.busy} onClick={close}>
            Volver
          </Button>
          <Button type="submit" disabled={op.busy || !supplier || !rows.length || !confirmed}>
            {op.busy ? 'Recibiendo…' : 'Confirmar recepción'}
            <PackageCheck size={17} />
          </Button>
        </div>
      </form>
      {pickSupplier && (
        <SupplierPicker
          onClose={() => setPickSupplier(false)}
          onSelect={(c) => {
            setSupplier(c);
            setPickSupplier(false);
            setConfirmed(false);
          }}
        />
      )}
      <Modal
        open={abandon}
        onClose={() => setAbandon(false)}
        title="¿Salir de esta recepción?"
        description="La compra todavía no se ha registrado. Se descartará lo que capturaste."
      >
        <div className="dialog-actions">
          <Button variant="outline" onClick={() => setAbandon(false)}>
            Seguir capturando
          </Button>
          <Button variant="destructive" onClick={onClose}>
            Descartar captura
          </Button>
        </div>
      </Modal>
    </Modal>
  );
}
function SupplierPicker({
  onClose,
  onSelect,
}: {
  onClose: () => void;
  onSelect: (contact: Contact) => void;
}) {
  const [search, setSearch] = useState('');
  const [create, setCreate] = useState(false);
  const list = useResource(
    () => api('contacts', { supplier: true, search, archived: false }),
    [search],
  );
  return (
    <Modal
      open
      onClose={onClose}
      title="Seleccionar proveedor"
      description="Busca por nombre o teléfono."
      className="records-dialog"
    >
      <div className="records-toolbar">
        <div className="records-search">
          <Search size={18} />
          <input
            autoFocus
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            aria-label="Buscar proveedor para compra"
            maxLength={300}
          />
        </div>
        <Button variant="outline" onClick={() => setCreate(true)}>
          <Plus size={16} /> Nuevo proveedor
        </Button>
      </div>
      <Failure error={list.error} retry={list.reload} />
      <DataGrid
        rows={list.data ?? []}
        caption="Seleccionar proveedor"
        columns={[
          { id: 'name', label: 'Proveedor', value: (c) => c.name },
          { id: 'phone', label: 'Teléfono', value: (c) => c.phone },
          {
            id: 'select',
            label: 'Elegir',
            value: (c) => c.id,
            sortable: false,
            render: (c) => (
              <Button variant="outline" size="sm" onClick={() => onSelect(c)}>
                Seleccionar
              </Button>
            ),
          },
        ]}
      />
      {create && (
        <ContactEditor
          supplier
          contact={newContact()}
          onClose={() => setCreate(false)}
          onSaved={onSelect}
        />
      )}
    </Modal>
  );
}
