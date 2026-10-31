import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Plus,
  Search,
  SlidersHorizontal,
  History,
  ArrowRightLeft,
  Pencil,
  RefreshCw,
  Package,
  Check,
  LoaderCircle,
  Barcode,
} from 'lucide-react';
import type { SortingState } from '@tanstack/react-table';
import { toast } from 'sonner';
import {
  api,
  can,
  normalizeError,
  type Actor,
  type Product,
  type ProductPage,
  type StockMovement,
  type UiFailure,
  type Bootstrap,
  type DocumentRequest,
} from '@/lib/api';
import { money, quantity, units, parseScaled } from '@/lib/utils';
import {
  Button,
  PageTitle,
  Field,
  Modal,
  Failure,
  Busy,
  useOperation,
  reportError,
} from '@/components/common';
import { DataGrid, type GridColumn } from '@/components/DataGrid';
import { DocumentDialog } from '@/components/DocumentViewer';

export const newProduct = (): Product => ({
  id: 0,
  sku: '',
  barcode: null,
  name: '',
  category: 'General',
  brand: '',
  size: '',
  aliases: '',
  unit: 0,
  costCents: null,
  priceCents: 0,
  wholesaleCents: 0,
  stockUnits: 0,
  minimumUnits: 0,
  taxBasisPoints: 1600,
  packName: 'Caja',
  packUnits: 10000,
  active: true,
});
export function Inventory({
  actor,
  business,
  filters = {},
  defaultTaxBasisPoints = 1600,
}: {
  actor: Actor;
  business: Bootstrap;
  filters?: Record<string, unknown>;
  defaultTaxBasisPoints?: number;
}) {
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState('');
  const [status, setStatus] = useState('active');
  const [page, setPage] = useState(0);
  const [sorting, setSorting] = useState<SortingState>([{ id: 'name', desc: false }]);
  const [data, setData] = useState<ProductPage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<UiFailure | null>(null);
  const [revision, setRevision] = useState(0);
  const [editing, setEditing] = useState<Product | null>(null);
  const [history, setHistory] = useState<Product | null>(null);
  const [adjusting, setAdjusting] = useState<Product | null>(null);
  const [labelProduct, setLabelProduct] = useState<Product | null>(null);
  const [labelDocument, setLabelDocument] = useState<DocumentRequest | null>(null);
  const editable = can(actor, 'catalog');
  const refresh = useCallback(() => setRevision((v) => v + 1), []);
  useEffect(() => {
    setPage(0);
    setStatus(typeof filters.status === 'string' ? filters.status : 'active');
    setSearch('');
    setCategory('');
    if (typeof filters.productId === 'number')
      void api('product', { id: filters.productId })
        .then((p) => {
          setSearch(p.sku);
          setStatus(p.active ? 'active' : 'archived');
        })
        .catch(reportError);
  }, [filters]);
  useEffect(() => {
    let active = true;
    setLoading(true);
    const timer = setTimeout(() => {
      void api('inventory', {
        search,
        category,
        status,
        page,
        sort: sorting[0]?.id ?? 'name',
        descending: sorting[0]?.desc ?? false,
      })
        .then((v) => {
          if (active) {
            setData(v);
            setError(null);
          }
        })
        .catch((e) => {
          if (active) setError(normalizeError(e));
        })
        .finally(() => {
          if (active) setLoading(false);
        });
    }, 150);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [search, category, status, page, sorting, revision]);
  const columns = useMemo<GridColumn<Product>[]>(
    () => [
      {
        id: 'name',
        label: 'Producto',
        value: (p) => p.name,
        render: (p) => (
          <button className="row-product" onClick={() => setEditing(p)}>
            <span className="product-mini-icon">
              <Package size={19} />
            </span>
            <span>
              <strong>{p.name}</strong>
              <small>
                {p.sku} · {p.category}
                {p.size ? ` · ${p.size}` : ''}
              </small>
            </span>
          </button>
        ),
      },
      {
        id: 'priceCents',
        label: 'Precio público',
        value: (p) => p.priceCents,
        align: 'right',
        render: (p) => <strong>{money(p.priceCents)}</strong>,
      },
      ...(editable
        ? [
            {
              id: 'costCents',
              label: 'Costo base',
              value: (p: Product) => p.costCents,
              align: 'right' as const,
              render: (p: Product) =>
                p.costCents === null ? (
                  <span className="muted">Sin capturar</span>
                ) : (
                  money(p.costCents)
                ),
            },
          ]
        : []),
      {
        id: 'stockUnits',
        label: 'Existencias',
        value: (p) => p.stockUnits,
        align: 'right',
        render: (p) => (
          <span
            className={`stock-status ${p.stockUnits === 0 ? 'empty' : p.stockUnits <= p.minimumUnits ? 'low' : ''}`}
          >
            <b>
              {quantity(p.stockUnits)} {units[p.unit]}
            </b>
            <small>
              {p.stockUnits === 0
                ? 'Agotado'
                : p.stockUnits <= p.minimumUnits
                  ? 'Stock bajo'
                  : `Mín. ${quantity(p.minimumUnits)}`}
            </small>
          </span>
        ),
      },
      {
        id: 'actions',
        label: 'Acciones',
        value: () => '',
        sortable: false,
        render: (p) => (
          <div className="row-actions">
            {editable && p.active && (
              <button
                className="icon-button"
                title="Imprimir etiquetas"
                aria-label={`Etiquetas de ${p.name}`}
                onClick={() => setLabelProduct(p)}
              >
                <Barcode size={16} />
              </button>
            )}
            {editable && (
              <button
                className="icon-button"
                title="Editar producto"
                aria-label={`Editar ${p.name}`}
                onClick={() => setEditing(p)}
              >
                <Pencil size={16} />
              </button>
            )}
            {can(actor, 'stock') && (
              <button
                className="icon-button"
                title="Ajustar existencias"
                aria-label={`Ajustar existencias de ${p.name}`}
                onClick={() => setAdjusting(p)}
              >
                <ArrowRightLeft size={16} />
              </button>
            )}
            <button
              className="icon-button"
              title="Ver movimientos"
              aria-label={`Ver movimientos de ${p.name}`}
              onClick={() => setHistory(p)}
            >
              <History size={16} />
            </button>
          </div>
        ),
      },
    ],
    [actor, editable],
  );
  return (
    <div className="page-scroll inventory-page">
      <PageTitle
        eyebrow="PRODUCTOS E INVENTARIO"
        title="Lo que tienes. Lo que necesitas."
        description="Encuentra un producto, revisa su existencia y mantén tu catálogo al día."
        actions={
          <>
            <Button variant="outline" onClick={refresh} aria-label="Actualizar inventario">
              <RefreshCw size={16} />
            </Button>
            {editable && (
              <Button
                onClick={() =>
                  setEditing({ ...newProduct(), taxBasisPoints: defaultTaxBasisPoints })
                }
              >
                <Plus size={17} />
                Nuevo producto
              </Button>
            )}
          </>
        }
      />
      <section className="surface inventory-surface" data-tour="inventory">
        <div className="table-toolbar">
          <div className="table-search">
            <Search size={18} />
            <input
              aria-label="Buscar en inventario"
              placeholder="Nombre, clave, código de barras o medida…"
              value={search}
              onChange={(e) => {
                setSearch(e.target.value);
                setPage(0);
              }}
              maxLength={200}
            />
            {loading && <LoaderCircle size={16} className="spin" />}
          </div>
          <label className="filter-select">
            <SlidersHorizontal size={16} />
            <select
              aria-label="Filtrar existencias"
              value={status}
              onChange={(e) => {
                setStatus(e.target.value);
                setPage(0);
              }}
            >
              <option value="active">Productos activos</option>
              <option value="low">Stock bajo y agotados</option>
              <option value="empty">Solo agotados</option>
              <option value="archived">Archivados</option>
            </select>
          </label>
          <select
            aria-label="Filtrar por categoría"
            value={category}
            onChange={(e) => {
              setCategory(e.target.value);
              setPage(0);
            }}
          >
            <option value="">Todas las categorías</option>
            {data?.categories.map((c) => (
              <option key={c.name}>{c.name}</option>
            ))}
          </select>
        </div>
        <Failure error={error} retry={refresh} />
        {data ? (
          <DataGrid
            rows={data.items}
            columns={columns}
            caption="Inventario de productos"
            emptyTitle="No encontramos productos con estos filtros."
            server={{
              page,
              pageSize: data.pageSize,
              total: data.total,
              sorting,
              onPage: setPage,
              onSorting: (next) => {
                setSorting(next);
                setPage(0);
              },
            }}
          />
        ) : (
          <Busy>Consultando el catálogo…</Busy>
        )}
        <div className="table-footnote">
          Precios de venta con impuestos incluidos. Los ajustes conservan su motivo y responsable.
        </div>
      </section>
      {editing && (
        <ProductEditor
          product={editing}
          editable={editable}
          categories={data?.categories.map((c) => c.name) ?? []}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            refresh();
            toast.success('Producto guardado.');
          }}
        />
      )}
      {adjusting && (
        <StockAdjust
          product={adjusting}
          onClose={() => setAdjusting(null)}
          onSaved={() => {
            setAdjusting(null);
            refresh();
            toast.success('Existencias actualizadas.');
          }}
        />
      )}
      {history && <StockHistory product={history} onClose={() => setHistory(null)} />}
      {labelProduct && (
        <LabelSetup
          product={labelProduct}
          onClose={() => setLabelProduct(null)}
          onPreview={(copies) => {
            setLabelDocument({ kind: 'labels', id: labelProduct.id, copies });
            setLabelProduct(null);
          }}
        />
      )}
      {labelDocument && (
        <DocumentDialog
          request={labelDocument}
          data={business}
          onClose={() => setLabelDocument(null)}
        />
      )}
    </div>
  );
}
function LabelSetup({
  product,
  onClose,
  onPreview,
}: {
  product: Product;
  onClose: () => void;
  onPreview: (copies: number) => void;
}) {
  const [copies, setCopies] = useState('16');
  const value = Number(copies);
  const valid = Number.isInteger(value) && value >= 1 && value <= 500;
  return (
    <Modal
      open
      onClose={onClose}
      title="Etiquetas listas para tu mostrador"
      description={product.name}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (valid) onPreview(value);
        }}
      >
        <p className="muted">
          Código, precio y logotipo del negocio. Hoja A4 con 16 etiquetas de 91 × 33 mm; imprime al
          100% y recorta.
        </p>
        <Field label="Cuántas etiquetas necesitas">
          <input
            type="number"
            min="1"
            max="500"
            step="1"
            value={copies}
            onChange={(e) => setCopies(e.target.value)}
            autoFocus
            required
          />
        </Field>
        <p className="muted">
          {product.barcode || product.sku} · {money(product.priceCents)} por {units[product.unit]}
        </p>
        <div className="dialog-actions">
          <Button type="button" variant="outline" onClick={onClose}>
            Volver
          </Button>
          <Button type="submit" disabled={!valid}>
            <Barcode size={17} />
            Ver etiquetas
          </Button>
        </div>
      </form>
    </Modal>
  );
}
function ProductEditor({
  product,
  editable,
  categories,
  onClose,
  onSaved,
}: {
  product: Product;
  editable: boolean;
  categories: string[];
  onClose: () => void;
  onSaved: () => void;
}) {
  const [p, setP] = useState(product);
  const [prices, setPrices] = useState({
    price: (p.priceCents / 100).toFixed(2),
    wholesale: (p.wholesaleCents / 100).toFixed(2),
    cost: p.costCents === null ? '' : (p.costCents / 100).toFixed(2),
    tax: (p.taxBasisPoints / 100).toString(),
    stock: (p.stockUnits / 10000).toString(),
    minimum: (p.minimumUnits / 10000).toString(),
    pack: (p.packUnits / 10000).toString(),
  });
  const op = useOperation();
  const patch = (values: Partial<Product>) => setP((v) => ({ ...v, ...values }));
  const amount = (key: keyof typeof prices, value: string) =>
    setPrices((v) => ({ ...v, [key]: value }));
  const step = [1, 2, 3].includes(p.unit) ? '0.0001' : '1';
  return (
    <Modal
      open
      onClose={() => {
        if (!op.busy) onClose();
      }}
      className="large-dialog"
      title={
        p.id ? (editable ? 'Ficha del producto' : 'Consulta de producto') : 'Agrega un producto'
      }
      description={
        p.id ? `${p.sku} · ${product.name}` : 'Define su unidad y precios para comenzar a venderlo.'
      }
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (!editable) return;
          void op.run(
            () =>
              api('saveProduct', {
                ...p,
                priceCents: parseScaled(prices.price),
                wholesaleCents: parseScaled(prices.wholesale),
                costCents: prices.cost.trim() === '' ? null : parseScaled(prices.cost),
                taxBasisPoints: parseScaled(prices.tax),
                stockUnits: parseScaled(prices.stock, 10000),
                minimumUnits: parseScaled(prices.minimum, 10000),
                packUnits: parseScaled(prices.pack, 10000),
              }),
            onSaved,
          );
        }}
      >
        <fieldset disabled={!editable || op.busy}>
          <div className="form-grid">
            <Field label="Descripción del producto">
              <input
                autoFocus
                required
                maxLength={200}
                value={p.name}
                onChange={(e) => patch({ name: e.target.value })}
              />
            </Field>
            <Field label="Clave interna">
              <input
                required
                maxLength={40}
                value={p.sku}
                onChange={(e) => patch({ sku: e.target.value })}
              />
            </Field>
            <Field label="Código de barras" hint="Puedes usar el lector para capturarlo.">
              <input
                maxLength={64}
                value={p.barcode ?? ''}
                onChange={(e) => patch({ barcode: e.target.value || null })}
              />
            </Field>
            <Field label="Categoría">
              <input
                required
                maxLength={80}
                list="product-categories"
                value={p.category}
                onChange={(e) => patch({ category: e.target.value })}
              />
              <datalist id="product-categories">
                {categories.map((c) => (
                  <option key={c} value={c} />
                ))}
              </datalist>
            </Field>
            <Field label="Marca">
              <input
                maxLength={200}
                value={p.brand}
                onChange={(e) => patch({ brand: e.target.value })}
              />
            </Field>
            <Field label="Medida o presentación comercial">
              <input
                maxLength={200}
                placeholder="Ej. Saco 50 kg, 1/2 pulgada…"
                value={p.size}
                onChange={(e) => patch({ size: e.target.value })}
              />
            </Field>
          </div>
          <div className="form-section-heading">
            <h3>Precios y unidad de venta</h3>
            <span>Todo se calcula en esta unidad base.</span>
          </div>
          <div className="form-grid">
            <Field label="Unidad base">
              <select value={p.unit} onChange={(e) => patch({ unit: Number(e.target.value) })}>
                {['Pieza', 'Metro', 'Kilogramo', 'Litro', 'Caja', 'Rollo'].map((name, i) => (
                  <option key={name} value={i}>
                    {name}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="IVA incluido (%)">
              <input
                required
                type="number"
                min="0"
                max="100"
                step="0.01"
                value={prices.tax}
                onChange={(e) => amount('tax', e.target.value)}
              />
            </Field>
            <Field label="Precio público (IVA incluido)">
              <input
                required
                type="number"
                min="0"
                step="0.01"
                value={prices.price}
                onChange={(e) => amount('price', e.target.value)}
              />
            </Field>
            <Field label="Precio de mayoreo (IVA incluido)">
              <input
                required
                type="number"
                min="0"
                step="0.01"
                value={prices.wholesale}
                onChange={(e) => amount('wholesale', e.target.value)}
              />
            </Field>
            {editable && (
              <Field label="Costo base sin IVA" hint="Déjalo vacío si aún no conoces el costo.">
                <input
                  type="number"
                  min="0"
                  step="0.01"
                  placeholder="Sin capturar"
                  value={prices.cost}
                  onChange={(e) => amount('cost', e.target.value)}
                />
              </Field>
            )}
            <Field label={`Existencia mínima (${units[p.unit]})`}>
              <input
                required
                type="number"
                min="0"
                step={step}
                value={prices.minimum}
                onChange={(e) => amount('minimum', e.target.value)}
              />
            </Field>
            {!p.id && (
              <Field
                label={`Existencia inicial (${units[p.unit]})`}
                hint="Después del alta, utiliza Ajustar existencias o registra una compra."
              >
                <input
                  required
                  type="number"
                  min="0"
                  step={step}
                  value={prices.stock}
                  onChange={(e) => amount('stock', e.target.value)}
                />
              </Field>
            )}
          </div>
          <details className="form-details">
            <summary>Presentación de compra y términos de búsqueda</summary>
            <div className="form-grid">
              <Field label="Presentación de compra">
                <input
                  maxLength={80}
                  value={p.packName}
                  onChange={(e) => patch({ packName: e.target.value })}
                />
              </Field>
              <Field label={`Cantidad de ${units[p.unit]} por presentación`}>
                <input
                  required
                  type="number"
                  min={step}
                  step={step}
                  value={prices.pack}
                  onChange={(e) => amount('pack', e.target.value)}
                />
              </Field>
            </div>
            <Field
              label="Otros nombres para encontrarlo"
              hint="Ejemplo: cinta teflón, sellador rosca, cinta plomero."
            >
              <input
                maxLength={1000}
                value={p.aliases}
                onChange={(e) => patch({ aliases: e.target.value })}
              />
            </Field>
          </details>
          <label className="checkbox-field">
            <input
              type="checkbox"
              checked={p.active}
              onChange={(e) => patch({ active: e.target.checked })}
            />
            <span>Producto activo para nuevas ventas</span>
          </label>
        </fieldset>
        <Failure error={op.error} />
        <div className="dialog-actions">
          <Button type="button" variant="outline" disabled={op.busy} onClick={onClose}>
            {editable ? 'Cancelar' : 'Cerrar'}
          </Button>
          {editable && (
            <Button disabled={op.busy}>
              <Check size={16} />
              {op.busy ? 'Guardando…' : 'Guardar producto'}
            </Button>
          )}
        </div>
      </form>
    </Modal>
  );
}
function StockAdjust({
  product,
  onClose,
  onSaved,
}: {
  product: Product;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [direction, setDirection] = useState(1);
  const [amount, setAmount] = useState('');
  const [reason, setReason] = useState('');
  const op = useOperation();
  const next = product.stockUnits + direction * parseScaled(amount, 10000);
  return (
    <Modal
      open
      onClose={() => {
        if (!op.busy) onClose();
      }}
      title="Ajustar existencias"
      description={`${product.sku} · ${product.name}`}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void op.run(
            () =>
              api('adjustStock', {
                id: product.id,
                delta: direction * parseScaled(amount, 10000),
                reason,
              }),
            onSaved,
          );
        }}
      >
        <div className="choice-group" role="group" aria-label="Tipo de ajuste">
          <button
            type="button"
            className={direction === 1 ? 'selected' : ''}
            aria-pressed={direction === 1}
            onClick={() => setDirection(1)}
          >
            Entrada de unidades
          </button>
          <button
            type="button"
            className={direction === -1 ? 'selected' : ''}
            aria-pressed={direction === -1}
            onClick={() => setDirection(-1)}
          >
            Salida de unidades
          </button>
        </div>
        <div className="form-grid">
          <Field label={`Cantidad (${units[product.unit]})`}>
            <input
              autoFocus
              required
              type="number"
              min={[1, 2, 3].includes(product.unit) ? '.0001' : '1'}
              step={[1, 2, 3].includes(product.unit) ? '.0001' : '1'}
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
            />
          </Field>
          <div className="adjust-summary">
            <span>
              {quantity(product.stockUnits)} {units[product.unit]} actuales
            </span>
            <strong className={next < 0 ? 'negative' : ''}>
              {quantity(next)} {units[product.unit]} después
            </strong>
          </div>
        </div>
        <Field label="Motivo del ajuste">
          <textarea
            required
            maxLength={300}
            placeholder="Ej. Corrección de conteo, mercancía dañada…"
            value={reason}
            onChange={(e) => setReason(e.target.value)}
          />
        </Field>
        <div className="notice">
          Para una entrada de proveedor, registra una compra: así también se actualiza el costo
          promedio.
        </div>
        <Failure error={op.error} />
        <div className="dialog-actions">
          <Button type="button" variant="outline" onClick={onClose} disabled={op.busy}>
            Cancelar
          </Button>
          <Button disabled={op.busy || next < 0}>
            {op.busy ? 'Guardando…' : 'Confirmar ajuste'}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
function StockHistory({ product, onClose }: { product: Product; onClose: () => void }) {
  const [rows, setRows] = useState<StockMovement[] | null>(null);
  const [error, setError] = useState<UiFailure | null>(null);
  useEffect(() => {
    void api('stockHistory', { id: product.id })
      .then(setRows)
      .catch((e) => setError(normalizeError(e)));
  }, [product.id]);
  const columns = useMemo<GridColumn<StockMovement>[]>(
    () => [
      { id: 'createdAt', label: 'Fecha', value: (r) => r.createdAt },
      {
        id: 'kind',
        label: 'Movimiento / motivo',
        value: (r) => r.kind,
        render: (r) => (
          <span className="two-line">
            <strong>{r.kind}</strong>
            <small>{r.reason || r.reference}</small>
          </span>
        ),
      },
      {
        id: 'deltaUnits',
        label: 'Cambio',
        value: (r) => r.deltaUnits,
        align: 'right',
        render: (r) => (
          <span className={r.deltaUnits < 0 ? 'negative' : 'positive'}>
            {r.deltaUnits > 0 ? '+' : ''}
            {quantity(r.deltaUnits)} {units[product.unit]}
          </span>
        ),
      },
      {
        id: 'balanceUnits',
        label: 'Existencia posterior',
        value: (r) => r.balanceUnits,
        align: 'right',
        render: (r) => `${quantity(r.balanceUnits)} ${units[product.unit]}`,
      },
    ],
    [product.unit],
  );
  return (
    <Modal
      open
      onClose={onClose}
      className="large-dialog"
      title="Historia de existencias"
      description={`${product.sku} · ${product.name} · Últimos 500 movimientos`}
    >
      <Failure error={error} />
      {rows ? (
        <DataGrid
          rows={rows}
          columns={columns}
          caption="Movimientos de inventario del producto"
          emptyTitle="Este producto todavía no tiene movimientos."
        />
      ) : (
        !error && <Busy />
      )}
    </Modal>
  );
}
