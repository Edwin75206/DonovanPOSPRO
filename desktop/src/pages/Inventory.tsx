import { useCallback, useEffect, useId, useMemo, useRef, useState, type AriaAttributes } from 'react';
import { StockIndicator } from '@/components/StockIndicator';
import { ProductAppearancePicker, ProductVisual } from '@/components/ProductVisual';
import {
  Plus,
  Minus,
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
  Trash2,
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
import { DeleteRecordDialog } from '@/components/DeleteRecordDialog';
import { useResource } from '@/lib/useResource';

const maxQuantity = 100_000_000;

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
  const [deleting, setDeleting] = useState<Product | null>(null);
  const [history, setHistory] = useState<Product | null>(null);
  const [adjusting, setAdjusting] = useState<Product | null>(null);
  const [labelProduct, setLabelProduct] = useState<Product | null>(null);
  const [labelDocument, setLabelDocument] = useState<DocumentRequest | null>(null);
  const editable = can(actor, 'catalog');
  const canAdjust = can(actor, 'stock');
  const adjustTrigger = useRef<HTMLButtonElement>(null);
  const openAdjustment = useCallback((product: Product, trigger: HTMLButtonElement) => {
    adjustTrigger.current = trigger;
    setAdjusting(product);
  }, []);
  const refresh = useCallback(() => setRevision((v) => v + 1), []);
  const afterDelete = () => {
    setDeleting(null);
    setEditing(null);
    if (page > 0 && data?.items.length === 1) setPage(page - 1);
    refresh();
    toast.success('Producto eliminado.');
  };
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
            <ProductVisual product={p} />
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
        render: (p) => <StockIndicator product={p} />,
      },
      {
        id: 'actions',
        label: 'Acciones',
        value: () => '',
        sortable: false,
        render: (p) => (
          <div className="row-actions inventory-actions">
            <Button
              variant="outline"
              size="sm"
              aria-label={`${editable ? 'Editar' : 'Ver ficha de'} ${p.name}`}
              onClick={() => setEditing(p)}
            >
              {editable && <Pencil size={15} />}
              {editable ? 'Editar' : 'Ver ficha'}
            </Button>
            {canAdjust && (
              <Button
                variant="outline"
                size="sm"
                aria-label={`Ajustar existencias de ${p.name}`}
                onClick={(event) => openAdjustment(p, event.currentTarget)}
              >
                <ArrowRightLeft size={15} />
                Ajustar existencias
              </Button>
            )}
            <Button
              variant="outline"
              size="sm"
              aria-label={`Ver movimientos de ${p.name}`}
              onClick={() => setHistory(p)}
            >
              <History size={15} />
              Historial
            </Button>
            {editable && (
              <Button variant="outline" size="sm" aria-label={`Eliminar producto ${p.name}`} onClick={() => setDeleting(p)}>
                <Trash2 size={15} />
                Eliminar
              </Button>
            )}
          </div>
        ),
      },
    ],
    [canAdjust, editable, openAdjustment],
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
          Precios de venta con impuestos incluidos. El historial conserva cada ajuste y su
          responsable.
        </div>
      </section>
      {editing && (
        <ProductEditor
          product={editing}
          editable={editable}
          categories={data?.categories.map((c) => c.name) ?? []}
          onAdjust={!editable && canAdjust ? (trigger) => openAdjustment(editing, trigger) : undefined}
          onLabels={editable && editing.active && editing.id ? () => setLabelProduct(editing) : undefined}
          onDeleted={editable && editing.id > 0 ? afterDelete : undefined}
          onClose={() => {
            if (!adjusting && !labelProduct && !labelDocument) setEditing(null);
          }}
          onSaved={() => {
            setEditing(null);
            refresh();
            toast.success('Producto guardado.');
          }}
        />
      )}
      {deleting && (
        <DeleteRecordDialog label="Eliminar producto" name={`${deleting.sku} · ${deleting.name}`}
          description="Se elimina la ficha y su carga inicial. Para un producto con operaciones registradas, usa la opción de archivar desde su ficha."
          onDelete={() => api('deleteProduct', { id: deleting.id })}
          onClose={() => setDeleting(null)} onDeleted={afterDelete} />
      )}
      {adjusting && (
        <StockAdjust
          product={adjusting}
          returnFocusRef={adjustTrigger}
          onClose={() => setAdjusting(null)}
          onSaved={(saved) => {
            setAdjusting(null);
            setEditing((product) => product?.id === saved.id ? saved : product);
            setData((page) => page && {
              ...page,
              items: page.items.map((product) => product.id === saved.id ? saved : product),
            });
            refresh();
            toast.success('Existencias actualizadas.', {
              description: `${quantity(saved.stockUnits)} ${units[saved.unit]} en inventario.`,
            });
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
export function ProductEditor({
  product,
  editable,
  categories,
  onAdjust,
  onLabels,
  onClose,
  onSaved,
  onDeleted,
}: {
  product: Product;
  editable: boolean;
  categories: string[];
  onAdjust?: (trigger: HTMLButtonElement) => void;
  onLabels?: () => void;
  onClose: () => void;
  onSaved: (saved: Product) => void;
  onDeleted?: () => void;
}) {
  const [p, setP] = useState(product);
  const [subdialog, setSubdialog] = useState<'category' | 'appearance' | 'delete' | 'box' | null>(null);
  const [addedCategories, setAddedCategories] = useState<string[]>([]);
  const [stockReason, setStockReason] = useState('');
  const activeHint = useId();
  const categoryRef = useRef<HTMLSelectElement>(null);
  const appearanceRef = useRef<HTMLButtonElement>(null);
  const [prices, setPrices] = useState({
    price: p.id ? (p.priceCents / 100).toFixed(2) : '',
    wholesale: p.wholesaleCents ? (p.wholesaleCents / 100).toFixed(2) : '',
    wholesaleMinimum: p.wholesaleMinimumUnits ? String(p.wholesaleMinimumUnits / 10000) : '',
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
  const options = [...new Set(['General', ...categories, ...addedCategories, p.category])].sort(
    (a, b) => a.localeCompare(b, 'es-MX'),
  );
  const stockUnits = parseScaled(prices.stock, 10000);
  const stockChanged = editable && !p.boxContent && p.id > 0 && prices.stock !== '' && stockUnits !== product.stockUnits;
  return (
    <Modal
      open
      onClose={() => {
        if (!op.busy && !subdialog) onClose();
      }}
      className="large-dialog product-dialog"
      title={
        p.id ? (editable ? 'Ficha del producto' : 'Consulta de producto') : 'Agrega un producto'
      }
      description={
        p.id ? `${p.sku} · ${product.name}` : 'Captura la categoría, el precio y las existencias.'
      }
    >
      {(onAdjust || onLabels || onDeleted) && (
        <div className="product-dialog-actions">
          {onAdjust && (
            <Button variant="outline" onClick={(event) => onAdjust(event.currentTarget)}>
              <ArrowRightLeft size={16} />
              Ajustar existencias
            </Button>
          )}
          {onLabels && (
            <Button variant="outline" onClick={onLabels} disabled={op.busy}>
              <Barcode size={16} />
              Imprimir etiquetas
            </Button>
          )}
          {onDeleted && (
            <Button variant="outline" disabled={op.busy} onClick={() => setSubdialog('delete')}>
              <Trash2 size={16} /> Eliminar producto
            </Button>
          )}
        </div>
      )}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          e.stopPropagation();
          if (!editable || subdialog) return;
          void op.run(
            () =>
              api('saveProduct', {
                ...p,
                sku: p.sku || (p.boxContent && p.barcode ? `CAJA-${p.barcode.trim().slice(0, 35)}` : ''),
                priceCents: parseScaled(prices.price),
                wholesaleCents: parseScaled(prices.wholesale),
                wholesaleMinimumUnits: prices.wholesaleMinimum === '' ? null : parseScaled(prices.wholesaleMinimum, 10000),
                costCents: prices.cost.trim() === '' ? null : parseScaled(prices.cost),
                taxBasisPoints: parseScaled(prices.tax),
                stockUnits: p.boxContent ? 0 : stockUnits,
                minimumUnits: parseScaled(prices.minimum, 10000),
                packUnits: parseScaled(prices.pack, 10000),
                inventoryAdjustment: stockChanged
                  ? { expectedStockUnits: product.stockUnits, reason: stockReason }
                  : undefined,
              }),
            onSaved,
          );
        }}
      >
        <fieldset disabled={!editable || op.busy}>
          <div className="form-grid">
            <Field label="Descripción del producto" hint="Nombre que aparece en el buscador y en el ticket.">
              <input
                autoFocus
                required
                maxLength={200}
                value={p.name}
                onChange={(e) => patch({ name: e.target.value })}
              />
            </Field>
            <Field label="Clave interna" hint="Clave única que usa el negocio para identificar el producto.">
              <input
                required={!p.boxContent}
                maxLength={40}
                value={p.sku}
                onChange={(e) => patch({ sku: e.target.value })}
              />
            </Field>
            <Field label="Código de barras" hint="Escanea la etiqueta o escribe el código para encontrarlo en caja.">
              <input
                required={!!p.boxContent}
                maxLength={64}
                value={p.barcode ?? ''}
                onChange={(e) => patch({ barcode: e.target.value || null })}
              />
            </Field>
            <Field label="Categoría" hint="Agrupa productos y permite filtrarlos en el catálogo.">
              <select
                ref={categoryRef}
                value={p.category}
                onChange={(e) => {
                  if (e.target.value === '') setSubdialog('category');
                  else patch({ category: e.target.value });
                }}
              >
                {options.map((category) => (
                  <option key={category} value={category}>
                    {category}
                  </option>
                ))}
                <option value="">+ Agregar categoría…</option>
              </select>
            </Field>
            <Field label="Marca" hint="Marca que aparece en el empaque o la etiqueta.">
              <input
                maxLength={200}
                value={p.brand}
                onChange={(e) => patch({ brand: e.target.value })}
              />
            </Field>
            <Field label="Medida o presentación comercial" hint="Tamaño, calibre o presentación, como saco de 50 kg.">
              <input
                maxLength={200}
                placeholder="Ej. Saco 50 kg, 1/2 pulgada…"
                value={p.size}
                onChange={(e) => patch({ size: e.target.value })}
              />
            </Field>
          </div>
          <div className="material-field">
            <ProductVisual product={p} />
            <div>
              <strong>Icono del producto</strong>
              <small>Ayuda a reconocer el producto en inventario y al buscarlo en caja.</small>
            </div>
            {editable && (
              <Button
                ref={appearanceRef}
                type="button"
                variant="outline"
                onClick={() => setSubdialog('appearance')}
              >
                Elegir icono y color
              </Button>
            )}
          </div>
          <div className="form-section-heading">
            <h3>Precios y unidad de venta</h3>
          </div>
          <div className="form-grid">
            <Field label="Unidad base" hint="Unidad en la que cuentas, vendes y asignas precios al producto.">
              <select value={p.unit} onChange={(e) => {
                const unit = Number(e.target.value);
                if (unit !== 4 && p.boxContent) amount('stock', '0');
                patch({ unit, boxContent: unit === 4 ? p.boxContent : null });
              }}>
                {['Pieza', 'Metro', 'Kilogramo', 'Litro', 'Caja', 'Rollo'].map((name, i) => (
                  <option key={name} value={i}>
                    {name}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="IVA incluido (%)" hint="Porcentaje de IVA que ya está incluido en los precios de venta.">
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
            <Field label={p.unit === 4 ? 'Precio de la caja (IVA incluido)' : 'Precio público (IVA incluido)'} hint="Importe por unidad en una venta normal.">
              <PriceInput
                required
                value={prices.price}
                onChange={(value) => amount('price', value)}
              />
            </Field>
            <Field label="Precio de mayoreo (IVA incluido)" hint="Importe por unidad cuando se vende a mayoreo.">
              <PriceInput
                value={prices.wholesale}
                onChange={(value) => amount('wholesale', value)}
              />
            </Field>
            <Field label={`Mayoreo desde (${units[p.unit]})`} hint="Aplica la tarifa al llegar a esta cantidad. Déjalo vacío para aplicarla manualmente.">
              <input type="number" min={step} max={maxQuantity} step={step}
                value={prices.wholesaleMinimum} onChange={(event) => amount('wholesaleMinimum', event.target.value)} />
            </Field>
            {editable && (
              <Field label="Costo base sin IVA" hint="Lo que cuesta una unidad antes de IVA. Puedes dejarlo vacío.">
                <PriceInput
                  placeholder="Sin capturar"
                  value={prices.cost}
                  onChange={(value) => amount('cost', value)}
                />
              </Field>
            )}
          </div>
          {p.unit === 4 && (
            <div className="form-grid">
              <Field label="Producto contenido" hint="Al cobrar una caja, se descontarán las unidades de este producto.">
                <div className="page-actions">
                  <Button type="button" variant="outline" aria-label={p.boxContent ? `Cambiar producto contenido: ${p.boxContent.name ?? ''}` : 'Seleccionar producto'} onClick={() => setSubdialog('box')}>
                    {p.boxContent?.name ?? 'Seleccionar producto'}
                  </Button>
                  {p.boxContent && (
                    <Button type="button" variant="outline" onClick={() => {
                      patch({ boxContent: null });
                      amount('stock', '0');
                    }}>
                      Quitar vínculo
                    </Button>
                  )}
                </div>
              </Field>
              {p.boxContent && (
                <Field label="Cantidad que contiene la caja" hint="Ejemplo 10 sacos de cemento por caja.">
                  <input type="number" required min={1} max={maxQuantity} step={1}
                    value={p.boxContent.quantity || ''}
                    onChange={(e) => patch({ boxContent: { ...p.boxContent!, quantity: Number(e.target.value) } })} />
                </Field>
              )}
            </div>
          )}
          <div className="form-section-heading">
            <h3>Inventario</h3>
          </div>
          <div className="form-grid">
            <Field
              label={`Existencia ${p.id ? 'actual' : 'inicial'} (${units[p.unit]})`}
              hint={p.boxContent ? 'Calculada a partir del inventario del producto contenido.' : p.id ? 'Cantidad total registrada en inventario en este momento.' : 'Unidades que tienes al dar de alta este producto.'}
            >
              <input
                required
                type="number"
                min="0"
                max={maxQuantity}
                step={step}
                readOnly={!!p.boxContent}
                value={p.boxContent
                  ? Math.floor((p.boxContent.stockUnits ?? 0) / (Math.max(1, p.boxContent.quantity) * 10000))
                  : editable ? prices.stock : (product.stockUnits / 10000).toString()}
                onFocus={(e) => e.currentTarget.select()}
                onChange={(e) => amount('stock', e.target.value)}
              />
            </Field>
            <Field
              label={`Existencia mínima (${units[p.unit]})`}
              hint="A esta cantidad o menos aparece Stock bajo. Si llega a cero, aparece Agotado."
            >
              <input
                required
                type="number"
                min="0"
                max={maxQuantity}
                step={step}
                value={prices.minimum}
                onFocus={(e) => e.currentTarget.select()}
                onChange={(e) => amount('minimum', e.target.value)}
              />
            </Field>
          </div>
          {stockChanged && (
            <Field label="Motivo del ajuste (opcional)" hint="Nota que queda en el historial para explicar el cambio.">
              <input
                maxLength={300}
                value={stockReason}
                onChange={(e) => setStockReason(e.target.value)}
              />
            </Field>
          )}
          <details className="form-details">
            <summary>Presentación de compra y términos de búsqueda</summary>
            <div className="form-grid">
              <Field label="Presentación de compra" hint="Cómo lo recibes del proveedor, por ejemplo en un paquete.">
                <input
                  maxLength={80}
                  value={p.packName}
                  onChange={(e) => patch({ packName: e.target.value })}
                />
              </Field>
              <Field label={`Cantidad de ${units[p.unit]} por presentación`} hint="Cuántas unidades base vienen en cada empaque.">
                <input
                  required
                  type="number"
                  min={step}
                  max={maxQuantity}
                  step={step}
                  value={prices.pack}
                  onChange={(e) => amount('pack', e.target.value)}
                />
              </Field>
            </div>
            <Field
              label="Otros nombres para encontrarlo"
              hint="Incluye sinónimos o abreviaturas, como cinta teflón o cinta plomero."
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
              aria-labelledby={`${activeHint}-label`}
              aria-describedby={activeHint}
              checked={p.active}
              onChange={(e) => patch({ active: e.target.checked })}
            />
            <span>
              <span id={`${activeHint}-label`}>Producto activo para nuevas ventas</span>
              <small id={activeHint}>Al desactivarlo se archiva y se conserva su historial.</small>
            </span>
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
      {subdialog === 'category' && (
        <CategoryEditor
          onClose={() => setSubdialog(null)}
          returnFocusRef={categoryRef}
          onAdd={(name) => {
            const existing = options.find(
              (option) => option.localeCompare(name, 'es-MX', { sensitivity: 'accent' }) === 0,
            );
            if (!existing) setAddedCategories((current) => [...current, name]);
            patch({ category: existing ?? name });
            setSubdialog(null);
          }}
        />
      )}
      {subdialog === 'appearance' && (
        <ProductAppearancePicker
          product={p}
          returnFocusRef={appearanceRef}
          onClose={() => setSubdialog(null)}
          onApply={(appearance) => {
            patch({ appearance });
            setSubdialog(null);
          }}
        />
      )}
      {subdialog === 'delete' && onDeleted && (
        <DeleteRecordDialog label="Eliminar producto" name={`${product.sku} · ${product.name}`}
          description="Se elimina la ficha y, si existe, su carga inicial. Las operaciones posteriores requieren archivar el producto."
          onDelete={() => api('deleteProduct', { id: product.id })}
          onClose={() => setSubdialog(null)} onDeleted={onDeleted} />
      )}
      {subdialog === 'box' && (
        <CatalogPicker title="Selecciona el producto de la caja" eligible={(item) => item.id !== p.id && !item.boxContent} onClose={() => setSubdialog(null)} onSelect={(item) => {
          patch({ boxContent: { productId: item.id, quantity: p.boxContent?.quantity || 1,
            name: item.name, unit: item.unit, stockUnits: item.stockUnits } });
          setSubdialog(null);
        }} />
      )}
    </Modal>
  );
}
export function CatalogPicker({ title, eligible, onClose, onSelect }: {
  title: string;
  eligible?: (item: Product) => boolean;
  onClose: () => void;
  onSelect: (item: Product) => void;
}) {
  const [search, setSearch] = useState('');
  const [page, setPage] = useState(0);
  const products = useResource(
    () => api('products', { search, category: '', status: 'active', page }),
    [search, page],
  );
  const items = products.data?.items.filter((item) => !eligible || eligible(item)) ?? [];
  return (
    <Modal open title={title} onClose={onClose}>
      <Field label="Buscar por nombre, clave o código">
        <input autoFocus value={search} maxLength={200} onChange={(e) => { setSearch(e.target.value); setPage(0); }} />
      </Field>
      <Failure error={products.error} />
      {products.busy ? <Busy /> : !products.error && (
        <div className="box-picker-results">
          {items.map((item) => (
            <Button key={item.id} type="button" variant="outline" onClick={() => onSelect(item)}>
              {item.name} · {item.sku}{item.barcode && ` · ${item.barcode}`}
            </Button>
          ))}
          {!items.length && <p className="muted">No hay productos disponibles en esta página.</p>}
        </div>
      )}
      {products.data && products.data.total > products.data.pageSize && (
        <div className="dialog-actions">
          <Button type="button" variant="outline" disabled={!page || products.busy} onClick={() => setPage(page - 1)}>Anterior</Button>
          <Button type="button" variant="outline" disabled={(page + 1) * products.data.pageSize >= products.data.total || products.busy} onClick={() => setPage(page + 1)}>Siguiente</Button>
        </div>
      )}
    </Modal>
  );
}
function CategoryEditor({
  onClose,
  onAdd,
  returnFocusRef,
}: {
  onClose: () => void;
  onAdd: (name: string) => void;
  returnFocusRef: React.RefObject<HTMLSelectElement | null>;
}) {
  const [name, setName] = useState('');
  return (
    <Modal
      open
      onClose={onClose}
      title="Agregar categoría"
      description="Se incorporará al catálogo al guardar el producto."
      returnFocusRef={returnFocusRef}
    >
      <form
        onSubmit={(event) => {
          event.preventDefault();
          const category = name.trim().normalize('NFC');
          if (category) onAdd(category);
        }}
      >
        <Field label="Nombre de la categoría" hint="Nombre que aparecerá en el filtro de inventario.">
          <input
            autoFocus
            required
            maxLength={80}
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </Field>
        <div className="dialog-actions">
          <Button type="button" variant="outline" onClick={onClose}>
            Cancelar
          </Button>
          <Button type="submit" disabled={!name.trim()}>
            Agregar y seleccionar
          </Button>
        </div>
      </form>
    </Modal>
  );
}
const groupPrice = (value: string) =>
  value.replace(/^\d+/, (pesos) => pesos.replace(/\B(?=(\d{3})+(?!\d))/g, ','));
function PriceInput({
  value,
  onChange,
  placeholder = '0.00',
  required,
  ...accessibility
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  required?: boolean;
} & AriaAttributes) {
  return (
    <input
      {...accessibility}
      inputMode="decimal"
      required={required}
      placeholder={placeholder}
      pattern="[0-9,]+(\.[0-9]{0,2})?|\.[0-9]{1,2}"
      value={groupPrice(value)}
      onFocus={(event) => event.currentTarget.select()}
      onBlur={() => onChange(value && value !== '.' ? Number(value).toFixed(2) : '')}
      onChange={(event) => {
        const input = event.currentTarget;
        const raw = input.value.replaceAll(',', '');
        if (!/^\d{0,10}(\.\d{0,2})?$/.test(raw)) return;
        let digits = input.value
          .slice(0, input.selectionStart ?? input.value.length)
          .replaceAll(',', '').length;
        const formatted = groupPrice(raw);
        let caret = 0;
        while (digits > 0 && caret < formatted.length) {
          if (formatted[caret++] !== ',') digits--;
        }
        input.value = formatted;
        input.setSelectionRange(caret, caret);
        onChange(raw);
      }}
    />
  );
}
function StockAdjust({
  product,
  returnFocusRef,
  onClose,
  onSaved,
}: {
  product: Product;
  returnFocusRef: React.RefObject<HTMLButtonElement | null>;
  onClose: () => void;
  onSaved: (product: Product) => void;
}) {
  const [direction, setDirection] = useState(1);
  const [amount, setAmount] = useState('');
  const [reason, setReason] = useState('');
  const op = useOperation();
  const unit = units[product.unit];
  const fraction = [1, 2, 3].includes(product.unit);
  const step = fraction ? '0.0001' : '1';
  const amountUnits = parseScaled(amount, 10000);
  const validAmount = (fraction ? /^\d*(\.\d{0,4})?$/ : /^\d+$/).test(amount)
    && amountUnits > 0 && amountUnits <= maxQuantity * 10000;
  const delta = direction * amountUnits;
  const next = product.stockUnits + delta;
  const validBalance = next >= 0 && next <= maxQuantity * 10000;
  return (
    <Modal
      open
      onClose={() => {
        if (!op.busy) onClose();
      }}
      title="Ajustar existencias"
      description={`${product.sku} · ${product.name}`}
      returnFocusRef={returnFocusRef}
      className="stock-adjust-dialog"
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (!validAmount || !validBalance) return;
          void op.run(
            () => api('adjustStock', { id: product.id, delta, reason }),
            onSaved,
          );
        }}
      >
        <fieldset disabled={op.busy}>
          <div className="choice-group stock-direction" role="group" aria-label="Tipo de ajuste">
            <button
              type="button"
              className={direction === 1 ? 'selected' : ''}
              aria-pressed={direction === 1}
              onClick={() => setDirection(1)}
            >
              <Plus size={16} />
              Agregar
            </button>
            <button
              type="button"
              className={direction === -1 ? 'selected' : ''}
              aria-pressed={direction === -1}
              onClick={() => setDirection(-1)}
            >
              <Minus size={16} />
              Quitar
            </button>
          </div>
          <Field
            label={`Cantidad a ${direction === 1 ? 'agregar' : 'quitar'} (${unit})`}
            hint={direction === 1 ? 'Indica cuántas unidades llegaron.' : 'Indica cuántas unidades quieres descontar.'}
          >
            <input
              autoFocus
              required
              type="number"
              inputMode={fraction ? 'decimal' : 'numeric'}
              min={step}
              max={maxQuantity}
              step={step}
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
            />
          </Field>
          <div className="adjust-summary">
            <div>
              <span>Existencia actual</span>
              <strong>{quantity(product.stockUnits)} {unit}</strong>
            </div>
            <div aria-live="polite">
              <span>Después del ajuste</span>
              <strong className={!validBalance ? 'negative' : ''}>
                {validAmount ? `${quantity(next)} ${unit}` : '—'}
              </strong>
            </div>
          </div>
          {validAmount && !validBalance && (
            <p className="stock-adjust-error negative" role="alert">
              {next < 0 ? 'La cantidad supera las existencias disponibles.' : 'El saldo supera el límite de inventario.'}
            </p>
          )}
          <details className="form-details stock-adjust-reason">
            <summary>Añadir motivo (opcional)</summary>
            <Field label="Motivo del ajuste" hint="La nota quedará en el historial de este producto.">
              <input
                maxLength={300}
                placeholder="Corrección de conteo, mercancía dañada…"
                value={reason}
                onChange={(e) => setReason(e.target.value)}
              />
            </Field>
          </details>
        </fieldset>
        <Failure error={op.error} />
        <div className="dialog-actions">
          <Button type="button" variant="outline" onClick={onClose} disabled={op.busy}>
            Cancelar
          </Button>
          <Button disabled={op.busy || !validAmount || !validBalance}>
            {op.busy ? 'Guardando…' : 'Guardar ajuste'}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
function StockHistory({ product, onClose }: { product: Product; onClose: () => void }) {
  const stockId = product.boxContent?.productId ?? product.id;
  const stockUnit = product.boxContent?.unit ?? product.unit;
  const [rows, setRows] = useState<StockMovement[] | null>(null);
  const [error, setError] = useState<UiFailure | null>(null);
  useEffect(() => {
    void api('stockHistory', { id: stockId })
      .then(setRows)
      .catch((e) => setError(normalizeError(e)));
  }, [stockId]);
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
            {quantity(r.deltaUnits)} {units[stockUnit]}
          </span>
        ),
      },
      {
        id: 'balanceUnits',
        label: 'Existencia posterior',
        value: (r) => r.balanceUnits,
        align: 'right',
        render: (r) => `${quantity(r.balanceUnits)} ${units[stockUnit]}`,
      },
    ],
    [stockUnit],
  );
  return (
    <Modal
      open
      onClose={onClose}
      className="large-dialog"
      title="Historia de existencias"
      description={product.boxContent ? `Inventario compartido con ${product.boxContent.name ?? 'el producto contenido'}` : `${product.sku} · ${product.name} · Últimos 500 movimientos`}
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
