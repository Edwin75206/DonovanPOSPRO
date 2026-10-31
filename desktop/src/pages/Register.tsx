import { useCallback, useEffect, useRef, useState } from 'react';
import { AnimatePresence, motion } from 'motion/react';
import {
  Search,
  ScanBarcode,
  Plus,
  Minus,
  Trash2,
  ShoppingBag,
  ArrowRight,
  Check,
  ReceiptText,
  Printer,
  Tag,
  SlidersHorizontal,
  ChevronLeft,
  ChevronRight,
  Hammer,
  PaintRoller,
  Zap,
  Construction,
  Wrench,
  ShieldCheck,
  ArrowUpRight,
  Pause,
  History,
} from 'lucide-react';
import {
  api,
  can,
  type Bootstrap,
  type Product,
  type ProductPage,
  type Charge,
  type SaleDetail,
  type Checkout,
  type CashOverview,
} from '@/lib/api';
import {
  Button,
  Modal,
  Field,
  Failure,
  useOperation,
  reportError,
  Empty,
} from '@/components/common';
import { money, quantity, units, parseScaled } from '@/lib/utils';
import { Logo } from './Auth';
import { toast } from 'sonner';
export interface CartRow {
  product: Product;
  quantityUnits: number;
  wholesale: boolean;
}
export interface Cart {
  rows: CartRow[];
  discountParts: number;
  requestId: string;
  customerId: number | null;
  quoteId: number | null;
  heldSaleId: number | null;
}
export const freshCart = (): Cart => ({
  rows: [],
  discountParts: 0,
  requestId: crypto.randomUUID(),
  customerId: null,
  quoteId: null,
  heldSaleId: null,
});
export function ProductArt({ category }: { category: string }) {
  const Icon =
    category === 'Construcción'
      ? Construction
      : category === 'Pintura'
        ? PaintRoller
        : category === 'Electricidad'
          ? Zap
          : category === 'Seguridad'
            ? ShieldCheck
            : category === 'Herramientas'
              ? Hammer
              : Wrench;
  return (
    <span
      className={`product-art art-${['Construcción', 'Pintura', 'Electricidad', 'Seguridad', 'Herramientas'].indexOf(category)}`}
    >
      <Icon strokeWidth={1.5} />
    </span>
  );
}
export function Register({
  data,
  cart,
  setCart,
  onCash,
  onSaved,
}: {
  data: Bootstrap;
  cart: Cart;
  setCart: (value: Cart | ((v: Cart) => Cart)) => void;
  onCash: () => void;
  onSaved: () => void;
}) {
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState('');
  const [page, setPage] = useState(0);
  const [products, setProducts] = useState<ProductPage>({
    items: [],
    total: 0,
    page: 0,
    pageSize: 60,
    categories: [],
  });
  const [searchBusy, setSearchBusy] = useState(false);
  const [charge, setCharge] = useState<Charge | null>(null);
  const [pricing, setPricing] = useState(false);
  const [cash, setCash] = useState<CashOverview | null>(null);
  const [checkout, setCheckout] = useState(false);
  const [lastSale, setLastSale] = useState<SaleDetail | null>(null);
  const [receipt, setReceipt] = useState(false);
  const [discount, setDiscount] = useState(false);
  const [highlight, setHighlight] = useState<number | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const queryId = useRef(0);
  const priceId = useRef(0);
  const [error, setError] = useState<ReturnType<typeof reportError> | null>(null);
  const items = cart.rows.map((r) => ({
    productId: r.product.id,
    quantityUnits: r.quantityUnits,
    wholesale: r.wholesale,
  }));
  const fetchProducts = useCallback(async () => {
    const current = ++queryId.current;
    setSearchBusy(true);
    try {
      const result = await api('products', { search, category, status: 'active', page });
      if (current === queryId.current) {
        setProducts(result);
        setError(null);
      }
    } catch (e) {
      if (current === queryId.current) setError(reportError(e));
    } finally {
      if (current === queryId.current) setSearchBusy(false);
    }
  }, [search, category, page]);
  useEffect(() => {
    const timer = setTimeout(() => void fetchProducts(), 140);
    return () => clearTimeout(timer);
  }, [fetchProducts]);
  useEffect(() => {
    void api('cashOverview').then(setCash).catch(reportError);
    searchRef.current?.focus();
  }, []);
  useEffect(() => {
    const current = ++priceId.current;
    if (!cart.rows.length) {
      setCharge(null);
      setPricing(false);
      return;
    }
    setPricing(true);
    const timer = setTimeout(() => {
      void api('price', { items, discountParts: cart.discountParts })
        .then((result) => {
          if (current === priceId.current) {
            setCharge(result);
            setError(null);
          }
        })
        .catch((e) => {
          if (current === priceId.current) setError(reportError(e));
        })
        .finally(() => {
          if (current === priceId.current) setPricing(false);
        });
    }, 80);
    return () => clearTimeout(timer);
  }, [JSON.stringify(items), cart.discountParts]);
  const add = useCallback(
    (product: Product) => {
      setCart((current) => {
        const existing = current.rows.find((x) => x.product.id === product.id);
        const next = (existing?.quantityUnits ?? 0) + 10000;
        if (next > product.stockUnits) {
          toast.error(
            `Solo hay ${quantity(product.stockUnits)} ${units[product.unit]} disponibles.`,
          );
          return current;
        }
        return {
          ...current,
          rows: existing
            ? current.rows.map((x) =>
                x.product.id === product.id ? { ...x, quantityUnits: next } : x,
              )
            : [...current.rows, { product, quantityUnits: 10000, wholesale: false }],
        };
      });
      setHighlight(product.id);
      setTimeout(() => setHighlight(null), 240);
      searchRef.current?.focus();
    },
    [setCart],
  );
  const scan = useCallback(
    async (code: string) => {
      try {
        const found = await api('products', {
          search: code,
          category: '',
          status: 'active',
          page: 0,
        });
        const exact = found.items.find(
          (p) => p.barcode === code || p.sku.toLowerCase() === code.toLowerCase(),
        );
        if (exact || found.total === 1) {
          add(exact ?? found.items[0]);
          setSearch('');
        } else if (found.total === 0) {
          toast.error('No encontramos un producto con ese código.', {
            action: { label: 'Buscar por nombre', onClick: () => searchRef.current?.select() },
          });
        } else {
          setSearch(code);
          searchRef.current?.focus();
        }
      } catch (e) {
        reportError(e);
      }
    },
    [add],
  );
  const beginCheckout = useCallback(async () => {
    if (!cart.rows.length || checkout) return;
    if (!cash?.open) {
      toast.info('Abre la caja antes de cobrar.', {
        action: { label: 'Ir a caja', onClick: onCash },
      });
      return;
    }
    try {
      setPricing(true);
      const current = await api('price', {
        items: cart.rows.map((r) => ({
          productId: r.product.id,
          quantityUnits: r.quantityUnits,
          wholesale: r.wholesale,
        })),
        discountParts: cart.discountParts,
      });
      setCharge(current);
      setCheckout(true);
    } catch (e) {
      reportError(e);
    } finally {
      setPricing(false);
    }
  }, [cart, cash, checkout, onCash]);
  useEffect(() => {
    let buffer = '';
    let last = 0;
    const key = (event: KeyboardEvent) => {
      if (event.key === 'F8') {
        event.preventDefault();
        void beginCheckout();
        return;
      }
      if (event.key === 'F3') {
        event.preventDefault();
        searchRef.current?.focus();
        searchRef.current?.select();
        return;
      }
      if (
        checkout ||
        receipt ||
        (event.target as HTMLElement)?.closest('input,textarea,select,[contenteditable=true]')
      )
        return;
      if (event.ctrlKey || event.altKey || event.metaKey) return;
      const now = performance.now();
      if (now - last > 70) buffer = '';
      last = now;
      if (event.key === 'Enter' && buffer.length >= 4) {
        event.preventDefault();
        void scan(buffer);
        buffer = '';
      } else if (event.key.length === 1) {
        buffer += event.key;
      }
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [beginCheckout, scan, checkout, receipt]);
  const changeQty = (id: number, value: number) => {
    setCart((c) => ({
      ...c,
      rows: c.rows.map((r) => (r.product.id === id ? { ...r, quantityUnits: value } : r)),
    }));
  };
  const total = charge?.totalCents ?? 0;
  return (
    <div className="register">
      <section className="register-catalog">
        <div className="register-heading">
          <div>
            <span className="eyebrow">VENTA DE MOSTRADOR</span>
            <h1>¿Qué vamos a llevar?</h1>
          </div>
          <button className={`cash-status ${cash?.open ? 'online' : 'offline'}`} onClick={onCash}>
            <span />
            {cash?.open ? 'Caja abierta' : 'Abrir caja'}
            <ArrowUpRight size={14} />
          </button>
        </div>
        <div className="scan-search" data-tour="product-search">
          <ScanBarcode size={24} />
          <input
            ref={searchRef}
            aria-label="Buscar o escanear producto"
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setPage(0);
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                void scan(search.trim());
              }
            }}
            placeholder="Escanea un código o busca por nombre, marca, medida…"
            autoComplete="off"
          />
          <kbd>F3</kbd>
          {search && (
            <button
              className="icon-button"
              aria-label="Limpiar búsqueda"
              onClick={() => {
                setSearch('');
                searchRef.current?.focus();
              }}
            >
              ×
            </button>
          )}
        </div>
        <div className="category-strip">
          <button
            className={!category ? 'active' : ''}
            onClick={() => {
              setCategory('');
              setPage(0);
            }}
          >
            Todos <span>{products.categories.reduce((s, c) => s + c.count, 0)}</span>
          </button>
          {products.categories.map((c) => (
            <button
              key={c.name}
              className={category === c.name ? 'active' : ''}
              onClick={() => {
                setCategory(c.name);
                setPage(0);
              }}
            >
              {c.name}
            </button>
          ))}
        </div>
        <div className="catalog-meta">
          <span>{searchBusy ? 'Buscando…' : `${products.total} productos disponibles`}</span>
          <span>Precios con IVA incluido</span>
        </div>
        <Failure error={error} retry={() => void fetchProducts()} />
        <div className="product-grid" aria-busy={searchBusy}>
          {products.items.map((p) => (
            <motion.button
              key={p.id}
              layout="position"
              transition={{ duration: 0.16 }}
              whileTap={{ scale: 0.985 }}
              className={`product-card ${highlight === p.id ? 'just-added' : ''} ${p.stockUnits <= 0 ? 'out-of-stock' : ''}`}
              onClick={() => add(p)}
              disabled={p.stockUnits <= 0}
              aria-label={`Agregar ${p.name}, ${money(p.priceCents)}`}
            >
              <div className="product-card-top">
                <ProductArt category={p.category} />
                <span className={`stock-pill ${p.stockUnits <= p.minimumUnits ? 'low' : ''}`}>
                  {p.stockUnits <= 0 ? 'Agotado' : `${quantity(p.stockUnits)} ${units[p.unit]}`}
                </span>
              </div>
              <span className="product-brand">{p.brand || p.category}</span>
              <strong>{p.name}</strong>
              <small>{p.size || p.sku}</small>
              <div className="product-card-bottom">
                <b>{money(p.priceCents)}</b>
                <span className="add-product">
                  <Plus size={17} />
                </span>
              </div>
            </motion.button>
          ))}
          {products.items.length === 0 && !searchBusy && (
            <Empty
              title={
                search ? 'No encontramos ese producto.' : 'Tu catálogo está listo para comenzar.'
              }
            >
              {search
                ? 'Prueba con una palabra, marca o medida diferente.'
                : 'Agrega productos o importa tu lista desde Excel.'}
            </Empty>
          )}
        </div>
        {products.total > 60 && (
          <div className="pagination">
            <span>
              Página {page + 1} de {Math.ceil(products.total / 60)}
            </span>
            <Button
              variant="outline"
              size="sm"
              disabled={page === 0}
              onClick={() => setPage(page - 1)}
            >
              <ChevronLeft size={15} />
              Anterior
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={(page + 1) * 60 >= products.total}
              onClick={() => setPage(page + 1)}
            >
              Siguiente
              <ChevronRight size={15} />
            </Button>
          </div>
        )}
      </section>
      <section className="cart-panel" data-tour="cart">
        <div className="cart-heading">
          <div className="cart-icon">
            <ShoppingBag size={20} />
          </div>
          <div>
            <h2>Ticket actual</h2>
            <span>
              {cart.rows.length} {cart.rows.length === 1 ? 'partida' : 'partidas'} · Venta en curso
            </span>
          </div>
          {cart.rows.length > 0 && (
            <button
              className="icon-button"
              aria-label="Vaciar carrito"
              onClick={() => {
                if (window.confirm('¿Vaciar este carrito? No se ha registrado ningún cobro.'))
                  setCart(freshCart());
              }}
            >
              <Trash2 size={17} />
            </button>
          )}
        </div>
        <div className="customer-picker">
          <span className="avatar small">PG</span>
          <div>
            <b>Público en general</b>
            <small>Cliente de mostrador</small>
          </div>
        </div>
        <div className="cart-rows">
          <AnimatePresence initial={false}>
            {cart.rows.map((row, index) => {
              const line = charge?.lines.find((l) => l.productId === row.product.id);
              return (
                <motion.div
                  layout
                  key={row.product.id}
                  className={`cart-row ${highlight === row.product.id ? 'just-added' : ''}`}
                  initial={{ opacity: 0, x: 12 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0, height: 0 }}
                  transition={{ duration: 0.16 }}
                >
                  <div className="cart-row-title">
                    <span>{String(index + 1).padStart(2, '0')}</span>
                    <strong>{row.product.name}</strong>
                    <button
                      aria-label={`Quitar ${row.product.name}`}
                      className="icon-button"
                      onClick={() =>
                        setCart((c) => ({
                          ...c,
                          rows: c.rows.filter((x) => x.product.id !== row.product.id),
                        }))
                      }
                    >
                      <Trash2 size={14} />
                    </button>
                  </div>
                  <div className="cart-row-price">
                    <span>
                      {money(row.wholesale ? row.product.wholesaleCents : row.product.priceCents)} /{' '}
                      {units[row.product.unit]}
                    </span>
                    {can(data.actor!, 'discount') && (
                      <button
                        onClick={() =>
                          setCart((c) => ({
                            ...c,
                            rows: c.rows.map((x) =>
                              x.product.id === row.product.id
                                ? { ...x, wholesale: !x.wholesale }
                                : x,
                            ),
                          }))
                        }
                      >
                        {row.wholesale ? 'Mayoreo' : 'Menudeo'}
                        <SlidersHorizontal size={11} />
                      </button>
                    )}
                  </div>
                  <div className="cart-row-controls">
                    <div className="quantity-control">
                      <button
                        aria-label={`Restar ${row.product.name}`}
                        onClick={() =>
                          changeQty(row.product.id, Math.max(10000, row.quantityUnits - 10000))
                        }
                      >
                        <Minus size={13} />
                      </button>
                      <input
                        type="number"
                        min={0.0001}
                        max={100000000}
                        step={[0, 4, 5].includes(row.product.unit) ? 1 : 0.0001}
                        aria-label={`Cantidad de ${row.product.name}`}
                        value={row.quantityUnits / 10000}
                        onChange={(e) =>
                          changeQty(row.product.id, parseScaled(e.target.value, 10000))
                        }
                      />
                      <button
                        aria-label={`Sumar ${row.product.name}`}
                        onClick={() => changeQty(row.product.id, row.quantityUnits + 10000)}
                      >
                        <Plus size={13} />
                      </button>
                    </div>
                    <b>{money(line?.totalCents ?? 0)}</b>
                  </div>
                </motion.div>
              );
            })}
          </AnimatePresence>
          {!cart.rows.length && (
            <div className="cart-empty">
              <ScanBarcode size={38} strokeWidth={1.25} />
              <h3>Todo comienza con un producto.</h3>
              <p>
                Escanea su código o selecciónalo
                <br />
                del catálogo para agregarlo aquí.
              </p>
              <span>El buscador está listo para tu lector.</span>
            </div>
          )}
        </div>
        <div className="cart-summary">
          <div className="cart-tools">
            {can(data.actor!, 'discount') && (
              <button onClick={() => setDiscount(!discount)}>
                <Tag size={14} />
                Descuento
              </button>
            )}
            <span>
              {cart.rows.length ? 'Revisa las cantidades antes de cobrar' : 'Venta local · MXN'}
            </span>
          </div>
          {discount && (
            <Field label="Descuento sobre el total (%)">
              <input
                type="number"
                min={0}
                max={100}
                step={0.01}
                value={cart.discountParts / 10000}
                onChange={(e) =>
                  setCart((c) => ({ ...c, discountParts: parseScaled(e.target.value, 10000) }))
                }
              />
            </Field>
          )}
          <dl>
            <div>
              <dt>Subtotal sin IVA</dt>
              <dd>{money(total - (charge?.taxCents ?? 0))}</dd>
            </div>
            <div>
              <dt>IVA incluido</dt>
              <dd>{money(charge?.taxCents ?? 0)}</dd>
            </div>
            {!!charge?.discountCents && (
              <div>
                <dt>Descuento aplicado</dt>
                <dd>−{money(charge.discountCents)}</dd>
              </div>
            )}
          </dl>
          <div className="cart-total">
            <span>
              Total a pagar<small>Pesos mexicanos</small>
            </span>
            <motion.strong
              key={total}
              initial={{ opacity: 0.6, y: 3 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.14 }}
            >
              {money(total)}
            </motion.strong>
          </div>
          <Button
            className="checkout-button"
            onClick={() => void beginCheckout()}
            disabled={!cart.rows.length || pricing}
            data-tour="checkout"
          >
            <span>
              {pricing ? 'Calculando…' : 'Cobrar venta'}
              <ArrowRight size={18} />
            </span>
            <kbd>F8</kbd>
          </Button>
        </div>
      </section>
      <Modal
        open={checkout && !!charge}
        onClose={() => setCheckout(false)}
        title="Un paso más y queda listo."
        description="Revisa el importe y registra cómo pagó el cliente."
        className="checkout-dialog"
      >
        {charge && (
          <PaymentForm
            charge={charge}
            cart={cart}
            onSuccess={(sale) => {
              setCheckout(false);
              setLastSale(sale);
              setCart(freshCart());
              void fetchProducts();
              void api('cashOverview').then(setCash).catch(reportError);
              onSaved();
            }}
          />
        )}
      </Modal>
      <Modal
        open={!!lastSale && !receipt}
        onClose={() => setLastSale(null)}
        title="Venta registrada"
        description="El cobro y las existencias se guardaron correctamente."
        className="success-dialog"
      >
        {lastSale && (
          <>
            <div className="success-check">
              <Check size={28} />
            </div>
            <span className="eyebrow">{lastSale.sale.folio}</span>
            <h2 className="sale-success-total">{money(lastSale.sale.totalCents)}</h2>
            <div className="change-banner">
              <span>Cambio para el cliente</span>
              <b>{money(lastSale.sale.changeCents)}</b>
            </div>
            <div className="success-actions">
              <Button variant="outline" onClick={() => setReceipt(true)}>
                <ReceiptText size={17} />
                Ver e imprimir ticket
              </Button>
              <Button
                onClick={() => {
                  setLastSale(null);
                  searchRef.current?.focus();
                }}
              >
                Siguiente venta
                <ArrowRight size={17} />
              </Button>
            </div>
          </>
        )}
      </Modal>
      <Modal
        open={receipt && !!lastSale}
        onClose={() => {
          setReceipt(false);
          setLastSale(null);
          searchRef.current?.focus();
        }}
        title="Comprobante de venta"
        description="La venta ya está guardada. Reimprimir no genera otro cobro."
        className="receipt-dialog"
      >
        {lastSale && (
          <>
            <Ticket data={data} detail={lastSale} />
            <Button
              className="no-print"
              onClick={() => {
                try {
                  window.print();
                } catch {
                  toast.error(
                    'No pudimos imprimir el ticket. La venta sí se guardó correctamente.',
                  );
                }
              }}
            >
              <Printer size={17} />
              Imprimir
            </Button>
          </>
        )}
      </Modal>
    </div>
  );
}
function PaymentForm({
  charge,
  cart,
  onSuccess,
}: {
  charge: Charge;
  cart: Cart;
  onSuccess: (sale: SaleDetail) => void;
}) {
  const [method, setMethod] = useState(0);
  const [received, setReceived] = useState(String(charge.totalCents / 100));
  const [reference, setReference] = useState('');
  const op = useOperation();
  const change = parseScaled(received) - charge.totalCents;
  return (
    <form
      className="payment-layout"
      onSubmit={(e) => {
        e.preventDefault();
        const request: Checkout = {
          requestId: cart.requestId,
          items: cart.rows.map((r) => ({
            productId: r.product.id,
            quantityUnits: r.quantityUnits,
            wholesale: r.wholesale,
          })),
          payments: [{ method, amountCents: charge.totalCents, reference }],
          tenderedCents: method === 0 ? parseScaled(received) : 0,
          discountParts: cart.discountParts,
          customerId: cart.customerId,
          dueDate: null,
          quoteId: cart.quoteId,
          heldSaleId: cart.heldSaleId,
        };
        void op.run(() => api('checkout', request), onSuccess);
      }}
    >
      <section className="payment-overview">
        <span className="eyebrow">TOTAL DE LA VENTA</span>
        <h2>{money(charge.totalCents)}</h2>
        <p>{cart.rows.length} partidas · IVA incluido</p>
        <div className="payment-items">
          {charge.lines.map((l) => (
            <div key={l.productId}>
              <span>
                {quantity(l.quantityUnits)} × {l.name}
              </span>
              <b>{money(l.totalCents)}</b>
            </div>
          ))}
        </div>
        <div className="payment-safe">
          <ShieldCheck size={18} />
          <span>La venta se confirma al registrar el pago.</span>
        </div>
      </section>
      <section className="payment-fields">
        <span className="field-label">Método de pago</span>
        <div className="payment-methods">
          {['Efectivo', 'Tarjeta', 'Transferencia'].map((name, i) => (
            <button
              type="button"
              className={method === i ? 'active' : ''}
              key={name}
              onClick={() => setMethod(i)}
            >
              {name}
              {method === i && <Check size={14} />}
            </button>
          ))}
        </div>
        {method === 0 ? (
          <>
            <Field label="Efectivo recibido">
              <div className="money-input">
                <span>$</span>
                <input
                  autoFocus
                  type="number"
                  min={charge.totalCents / 100}
                  step="0.01"
                  required
                  value={received}
                  onFocus={(e) => e.currentTarget.select()}
                  onChange={(e) => setReceived(e.target.value)}
                />
                <span>MXN</span>
              </div>
            </Field>
            <button
              type="button"
              className="text-button"
              onClick={() => setReceived(String(charge.totalCents / 100))}
            >
              Importe exacto
            </button>
            <div className={`change-banner ${change < 0 ? 'negative' : ''}`}>
              <span>{change < 0 ? 'Falta por recibir' : 'Cambio'}</span>
              <b>{money(Math.abs(change))}</b>
            </div>
          </>
        ) : (
          <Field label="Referencia (opcional)">
            <input
              value={reference}
              onChange={(e) => setReference(e.target.value)}
              placeholder="Folio de la operación"
              maxLength={200}
            />
          </Field>
        )}
        <Failure error={op.error} />
        <Button type="submit" className="wide" disabled={op.busy || (method === 0 && change < 0)}>
          {op.busy ? 'Registrando venta…' : 'Confirmar pago'}
          <Check size={18} />
        </Button>
        <p className="payment-hint">Si la impresión falla, podrás reimprimir desde el historial.</p>
      </section>
    </form>
  );
}
export function Ticket({ data, detail }: { data: Bootstrap; detail: SaleDetail }) {
  return (
    <div className="ticket-paper" style={{ width: `${data.settings?.ticketWidthMm ?? 80}mm` }}>
      <Logo data={data} />
      <h3>{data.settings?.storeName}</h3>
      <p>
        {data.settings?.address}
        <br />
        {data.settings?.phone}
      </p>
      <p>
        <b>{detail.sale.folio}</b>
        <br />
        {detail.sale.createdAt}
        <br />
        Atendió: {detail.sale.cashier}
        <br />
        {detail.sale.customerName}
      </p>
      <hr />
      {detail.lines.map((l) => (
        <div className="ticket-line" key={l.id}>
          <strong>{l.name}</strong>
          <span>
            {quantity(l.quantityUnits)} × {money(l.priceCents)}
          </span>
          <b>{money(l.totalCents)}</b>
        </div>
      ))}
      <hr />
      <div className="ticket-total">
        <b>TOTAL</b>
        <b>{money(detail.sale.totalCents)}</b>
      </div>
      <div className="ticket-total">
        <span>IVA incluido</span>
        <span>{money(detail.sale.taxCents)}</span>
      </div>
      {detail.payments.map((p, i) => (
        <div className="ticket-total" key={i}>
          <span>{['Efectivo', 'Tarjeta', 'Transferencia', 'Crédito'][p.method]}</span>
          <span>{money(p.amountCents)}</span>
        </div>
      ))}
      <div className="ticket-total">
        <span>Cambio</span>
        <span>{money(detail.sale.changeCents)}</span>
      </div>
      <hr />
      <p>
        {data.settings?.ticketFooter}
        <br />
        <small>Comprobante de venta · No es CFDI</small>
      </p>
    </div>
  );
}
