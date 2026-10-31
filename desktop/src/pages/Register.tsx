import { PaymentForm } from './PaymentForm';
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
  LayoutGrid,
  List,
  PackageCheck,
  AlertTriangle,
  PackageX,
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
import { DocumentDialog, autoPrintSale } from '@/components/DocumentViewer';
import { toast } from 'sonner';
import { CustomerPicker, SaveDraft, HeldSales } from './CartActions';
import { useVisual } from '@/lib/appearance';
import { QuantityEditor } from '@/components/QuantityEditor';
import { RegisterCash, RegisterRecent } from './RegisterTools';
import { addQuantity, previewAmounts, ScanBuffer, stockState, validQuantity } from '@/lib/register';
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
  editingQuote: { id: number; validUntil: string; notes: string } | null;
  heldSaleId: number | null;
  customerName: string | null;
  sourceLabel: string | null;
}
export const freshCart = (): Cart => ({
  rows: [],
  discountParts: 0,
  requestId: crypto.randomUUID(),
  customerId: null,
  quoteId: null,
  editingQuote: null,
  heldSaleId: null,
  customerName: null,
  sourceLabel: null,
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
  onHistory,
  onQuotes,
}: {
  data: Bootstrap;
  cart: Cart;
  setCart: (value: Cart | ((v: Cart) => Cart)) => void;
  onCash: () => void;
  onSaved: () => void;
  onHistory: () => void;
  onQuotes: () => void;
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
  const [quoteDocument, setQuoteDocument] = useState<number | null>(null);
  const [discount, setDiscount] = useState(false);
  const [highlight, setHighlight] = useState<number | null>(null);
  const [customerPicker, setCustomerPicker] = useState(false);
  const [saveDraft, setSaveDraft] = useState<'held' | 'quote' | null>(null);
  const [held, setHeld] = useState(false);
  const [clear, setClear] = useState(false);
  const [quickCash, setQuickCash] = useState(false);
  const [recent, setRecent] = useState(false);
  const [showBreakdown, setShowBreakdown] = useState(false);
  const [checkoutBusy, setCheckoutBusy] = useState(false);
  const [view, setView] = useState<'cards' | 'list'>('cards');
  const [selection, setSelection] = useState(-1);
  const [invalidRows, setInvalidRows] = useState<Set<number>>(new Set());
  const [scanBusy, setScanBusy] = useState(0);
  const scanner = useRef(new ScanBuffer());
  const [addedMessage, setAddedMessage] = useState('');
  const cartRef = useRef(cart);
  cartRef.current = cart;
  const scanQueue = useRef(Promise.resolve());
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  const updateCart = useCallback(
    (update: (current: Cart) => Cart) => {
      const next = update(cartRef.current);
      cartRef.current = next;
      setCart(next);
    },
    [setCart],
  );
  const visual = useVisual();
  const fixedQuote = cart.quoteId !== null;
  const searchRef = useRef<HTMLInputElement>(null);
  const catalogRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    catalogRef.current?.querySelector('.keyboard-selected')?.scrollIntoView?.({ block: 'nearest' });
  }, [selection]);
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
    setSelection(-1);
    const timer = setTimeout(() => void fetchProducts(), search ? 90 : 0);
    return () => {
      clearTimeout(timer);
      queryId.current++;
    };
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
      setAddedMessage('');
      return;
    }
    setPricing(true);
    setCharge(null);
    const timer = setTimeout(() => {
      void api('price', { items, discountParts: cart.discountParts, quoteId: cart.quoteId })
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
  }, [JSON.stringify(items), cart.discountParts, cart.quoteId]);
  const add = useCallback(
    (product: Product) => {
      if (fixedQuote) {
        toast.info(
          'Esta venta conserva las partidas de la cotización. Finalízala antes de iniciar otra.',
        );
        return;
      }
      const existing = cartRef.current.rows.find((x) => x.product.id === product.id);
      const next = addQuantity(product, existing?.quantityUnits);
      if (!product.active || next === null) {
        toast.error(
          product.stockUnits <= 0
            ? `${product.name}: agotado.`
            : `Solo hay ${quantity(product.stockUnits)} ${units[product.unit]} disponibles; revisa tu ticket.`,
        );
        return;
      }
      updateCart((current) => {
        return {
          ...current,
          rows: existing
            ? current.rows.map((x) =>
                x.product.id === product.id ? { ...x, product, quantityUnits: next } : x,
              )
            : [...current.rows, { product, quantityUnits: next, wholesale: false }],
        };
      });
      setAddedMessage(`${product.name} · ${quantity(next)} ${units[product.unit]} en tu ticket`);
      setHighlight(product.id);
      setTimeout(() => setHighlight((current) => (current === product.id ? null : current)), 420);
      const active = document.activeElement;
      if (
        active === searchRef.current ||
        !active?.matches('input,textarea,select,[contenteditable=true]')
      )
        searchRef.current?.focus();
    },
    [updateCart, fixedQuote],
  );
  const scan = useCallback(
    (code: string) => {
      if (!code.trim()) return;
      setSearch('');
      setPage(0);
      setScanBusy((n) => n + 1);
      scanQueue.current = scanQueue.current
        .then(async () => {
          if (!mounted.current || document.querySelector('[role=dialog]')) return;
          try {
            const found = await api('productForCode', { code: code.trim() });
            if (!mounted.current || document.querySelector('[role=dialog]')) return;
            add(found);
          } catch (e) {
            if (mounted.current) reportError(e);
          }
        })
        .finally(() => {
          if (mounted.current) setScanBusy((n) => Math.max(0, n - 1));
        });
    },
    [add],
  );
  const beginCheckout = useCallback(async () => {
    if (
      !cart.rows.length ||
      checkout ||
      pricing ||
      scanBusy ||
      cart.rows.some((r) => invalidRows.has(r.product.id))
    )
      return;
    if (cart.editingQuote) {
      setSaveDraft('quote');
      return;
    }
    if (!cash?.open) {
      setQuickCash(true);
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
        quoteId: cart.quoteId,
      });
      setCharge(current);
      setCheckout(true);
    } catch (e) {
      reportError(e);
    } finally {
      setPricing(false);
    }
  }, [cart, cash, checkout, pricing, scanBusy, invalidRows, onCash]);
  useEffect(() => {
    const buffer = scanner.current;
    const key = (event: KeyboardEvent) => {
      if (
        document.querySelector('[role=dialog]') ||
        document.body.classList.contains('driver-active')
      ) {
        buffer.reset();
        return;
      }
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
      const editable = (event.target as HTMLElement)?.closest(
        'input,textarea,select,[contenteditable=true]',
      );
      const allowed = !checkout && !receipt && (!editable || event.target === searchRef.current);
      const code = buffer.key(
        event.key,
        performance.now(),
        allowed,
        event.ctrlKey || event.altKey || event.metaKey || event.isComposing || event.repeat,
      );
      if (code) {
        event.preventDefault();
        event.stopPropagation();
        scan(code);
      }
    };
    window.addEventListener('keydown', key, true);
    const reset = () => buffer.reset();
    window.addEventListener('blur', reset);
    window.addEventListener('focusin', reset);
    return () => {
      window.removeEventListener('keydown', key, true);
      window.removeEventListener('blur', reset);
      window.removeEventListener('focusin', reset);
    };
  }, [beginCheckout, scan, checkout, receipt]);
  const changeQty = (id: number, value: number) => {
    const product = cartRef.current.rows.find((r) => r.product.id === id)?.product;
    if (!product || !validQuantity(product, value)) return;
    updateCart((c) => ({
      ...c,
      rows: c.rows.map((r) => (r.product.id === id ? { ...r, quantityUnits: value } : r)),
    }));
  };
  const preview = !fixedQuote
    ? previewAmounts(cart.rows, Math.min(1_000_000, Math.max(0, cart.discountParts)))
    : null;
  const total = charge?.totalCents ?? preview?.totalCents ?? 0;
  const pendingQuantity = cart.rows.some((r) => invalidRows.has(r.product.id));
  return (
    <div className="register">
      <section className="register-catalog">
        <div className="register-heading">
          <div>
            <span className="eyebrow">VENTA DE MOSTRADOR</span>
            <h1>Todo listo para vender.</h1>
          </div>
          <button
            data-tour="cash-status"
            className={`cash-status ${cash?.open ? 'online' : 'offline'}`}
            onClick={() => setQuickCash(true)}
          >
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
              setSearchBusy(true);
              setPage(0);
              setSelection(-1);
            }}
            onKeyDown={(e) => {
              if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
                e.preventDefault();
                setSelection((n) =>
                  Math.max(
                    0,
                    Math.min(products.items.length - 1, n + (e.key === 'ArrowDown' ? 1 : -1)),
                  ),
                );
              }
              if (e.key === 'Enter') {
                e.preventDefault();
                if (selection >= 0 && products.items[selection] && !searchBusy)
                  add(products.items[selection]);
                else if (
                  products.items.some(
                    (p) =>
                      p.barcode === search.trim() ||
                      p.sku.toLowerCase() === search.trim().toLowerCase(),
                  )
                )
                  scan(search);
                else if (products.items.length) {
                  setSelection(0);
                  toast.info(
                    'Selecciona el producto con las flechas y pulsa Enter para agregarlo.',
                  );
                }
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
        <div className="category-strip" aria-label="Categorías de productos">
          <button
            aria-pressed={!category}
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
              aria-pressed={category === c.name}
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
          <span>
            {searchBusy
              ? 'Buscando…'
              : `${products.total} ${products.total === 1 ? 'producto' : 'productos'}`}{' '}
            · IVA incluido
          </span>
          <div className="catalog-view" aria-label="Vista del catálogo">
            <button
              aria-label="Ver tarjetas"
              aria-pressed={view === 'cards'}
              onClick={() => setView('cards')}
            >
              <LayoutGrid size={16} />
            </button>
            <button
              aria-label="Ver lista compacta"
              aria-pressed={view === 'list'}
              onClick={() => setView('list')}
            >
              <List size={17} />
            </button>
          </div>
        </div>
        <div className="scan-feedback" role="status" aria-live="polite">
          <Check size={14} />
          {scanBusy
            ? 'Leyendo código…'
            : addedMessage || 'Escanea y sigue · Flechas y Enter para elegir · F8 para cobrar'}
        </div>
        <Failure error={error} retry={() => void fetchProducts()} />
        <div ref={catalogRef} className={`product-grid catalog-${view}`} aria-busy={searchBusy}>
          {products.items.map((p, productIndex) => {
            const inCart = cart.rows.find((r) => r.product.id === p.id)?.quantityUnits ?? 0;
            const stock = stockState(p, inCart);
            const StatusIcon =
              stock.state === 'empty'
                ? PackageX
                : stock.state === 'low'
                  ? AlertTriangle
                  : PackageCheck;
            return (
              <motion.button
                key={p.id}
                layout="position"
                transition={{ duration: visual.duration }}
                whileTap={{ scale: visual.duration ? 0.985 : 1 }}
                className={`product-card ${selection === productIndex ? 'keyboard-selected' : ''} ${highlight === p.id ? 'just-added' : ''} ${p.stockUnits <= 0 ? 'out-of-stock' : ''}`}
                onClick={() => add(p)}
                disabled={stock.available <= 0}
                aria-label={`${p.name}, ${money(p.priceCents)} por ${units[p.unit]}, ${stock.label}. Agregar`}
              >
                <div className="product-card-top">
                  <ProductArt category={p.category} />
                  {!!inCart && <span className="in-ticket">{quantity(inCart)} en ticket</span>}
                </div>
                <span className="product-brand">{p.brand || p.category}</span>
                <strong>{p.name}</strong>
                <small>{p.size || p.sku}</small>
                <span className={`product-stock stock-${stock.state}`}>
                  <StatusIcon size={13} />
                  {stock.label}
                </span>
                <div className="product-card-bottom">
                  <span>
                    <b>{money(p.priceCents)}</b>
                    <small> / {units[p.unit]}</small>
                  </span>
                  <span className="add-product">
                    <Plus size={15} />
                    <span>Agregar</span>
                  </span>
                </div>
              </motion.button>
            );
          })}
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
                setClear(true);
              }}
            >
              <Trash2 size={17} />
            </button>
          )}
        </div>
        <button
          className="customer-picker"
          disabled={fixedQuote}
          onClick={() => setCustomerPicker(true)}
        >
          <span className="avatar small">PG</span>
          <div>
            <b>{cart.customerName || 'Público en general'}</b>
            <small>
              {fixedQuote
                ? `Cotización ${cart.sourceLabel}`
                : cart.customerId
                  ? 'Cambiar cliente'
                  : 'Seleccionar cliente'}
            </small>
          </div>
        </button>
        {fixedQuote && (
          <p className="quote-lock">Precios y cantidades acordados · {cart.sourceLabel}</p>
        )}
        {cart.editingQuote && (
          <p className="quote-lock">
            Editando {cart.sourceLabel} · Precios actuales. Guarda los cambios antes de vender.
          </p>
        )}
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
                  transition={{ duration: visual.duration }}
                >
                  <div className="cart-row-title">
                    <span>{String(index + 1).padStart(2, '0')}</span>
                    <strong>{row.product.name}</strong>
                    <button
                      aria-label={`Quitar ${row.product.name}`}
                      className="icon-button"
                      disabled={fixedQuote}
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
                      {money(
                        line?.priceCents ??
                          (row.wholesale ? row.product.wholesaleCents : row.product.priceCents),
                      )}{' '}
                      / {units[row.product.unit]}
                    </span>
                    {can(data.actor!, 'discount') && !fixedQuote && (
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
                    <QuantityEditor
                      product={row.product}
                      value={row.quantityUnits}
                      disabled={fixedQuote}
                      onChange={(value) => changeQty(row.product.id, value)}
                      onValidity={(valid) =>
                        setInvalidRows((current) => {
                          const next = new Set(current);
                          if (valid) next.delete(row.product.id);
                          else next.add(row.product.id);
                          return next;
                        })
                      }
                    />
                    <b>
                      {line
                        ? money(line.totalCents)
                        : preview
                          ? money(preview.lineCents[index])
                          : '…'}
                    </b>
                  </div>
                  {invalidRows.has(row.product.id) && (
                    <small className="quantity-warning">
                      Indica {row.product.unit === 0 ? 'piezas enteras' : 'una cantidad válida'};
                      máximo {quantity(row.product.stockUnits)} {units[row.product.unit]}.
                    </small>
                  )}
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
            {can(data.actor!, 'discount') && !fixedQuote && (
              <button onClick={() => setDiscount(!discount)}>
                <Tag size={14} />
                Descuento
              </button>
            )}
            <button
              className="breakdown-toggle"
              onClick={() => setShowBreakdown(!showBreakdown)}
              aria-expanded={showBreakdown}
            >
              Desglose
            </button>
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
          <dl className={showBreakdown ? 'breakdown-open' : ''}>
            <div>
              <dt>Subtotal sin IVA</dt>
              <dd>{charge ? money(total - charge.taxCents) : cart.rows.length ? '…' : money(0)}</dd>
            </div>
            <div>
              <dt>IVA incluido</dt>
              <dd>{charge ? money(charge.taxCents) : cart.rows.length ? '…' : money(0)}</dd>
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
              key={charge ? total : 'pending'}
              initial={{ opacity: 0.6, y: 3 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: visual.duration }}
            >
              {charge || preview ? money(total) : cart.rows.length ? '…' : money(0)}
            </motion.strong>
          </div>
          <Button
            className="checkout-button"
            onClick={() => void beginCheckout()}
            disabled={!cart.rows.length || pricing || !charge || scanBusy > 0 || pendingQuantity}
            data-tour="checkout"
          >
            <span>
              {pendingQuantity
                ? 'Revisa la cantidad'
                : pricing
                  ? 'Validando…'
                  : cart.editingQuote
                    ? 'Guardar cotización'
                    : 'Cobrar venta'}
              <ArrowRight size={18} />
            </span>
            <kbd>F8</kbd>
          </Button>
          <div className="cart-secondary-actions" data-tour="cart-tools">
            <button
              disabled={
                !cart.rows.length ||
                fixedQuote ||
                !!cart.editingQuote ||
                pricing ||
                !charge ||
                pendingQuantity
              }
              onClick={() => setSaveDraft('held')}
            >
              <Pause size={14} /> Guardar
            </button>
            <button
              disabled={!cart.rows.length || fixedQuote || pricing || !charge || pendingQuantity}
              onClick={() => setSaveDraft('quote')}
            >
              <ReceiptText size={14} /> {cart.editingQuote ? 'Vigencia y notas' : 'Cotizar'}
            </button>
            <button onClick={() => setHeld(true)}>
              <History size={14} /> En espera
            </button>
          </div>
          <div className="cart-links">
            <button onClick={() => setRecent(true)}>Ventas recientes</button>
            <button onClick={onQuotes}>Mis cotizaciones</button>
          </div>
        </div>
      </section>
      {quickCash && (
        <RegisterCash
          cash={cash}
          onClose={() => setQuickCash(false)}
          onManage={onCash}
          onSaved={(value) => {
            setCash(value);
            setQuickCash(false);
            searchRef.current?.focus();
          }}
        />
      )}
      {recent && (
        <RegisterRecent data={data} onClose={() => setRecent(false)} onHistory={onHistory} />
      )}
      <Modal
        open={checkout && !!charge}
        onClose={() => {
          if (!checkoutBusy) setCheckout(false);
        }}
        title="Cobrar y entregar."
        description="Confirma el pago. Tu ticket permanece en esta pantalla."
        className="checkout-dialog"
      >
        {charge && (
          <PaymentForm
            charge={charge}
            cart={cart}
            allowCredit={can(data.actor!, 'credit')}
            onBusy={setCheckoutBusy}
            onSuccess={(sale) => {
              setCheckout(false);
              setLastSale(sale);
              setCart(freshCart());
              autoPrintSale(data, sale.sale.id);
              void fetchProducts();
              void api('cashOverview').then(setCash).catch(reportError);
              onSaved();
            }}
          />
        )}
      </Modal>
      {customerPicker && (
        <CustomerPicker
          actor={data.actor!}
          onClose={() => setCustomerPicker(false)}
          onSelect={(customer) => {
            setCart((c) => ({
              ...c,
              customerId: customer?.id ?? null,
              customerName: customer?.name ?? null,
            }));
            setCustomerPicker(false);
          }}
        />
      )}
      {saveDraft && (
        <SaveDraft
          cart={cart}
          kind={saveDraft}
          onClose={() => setSaveDraft(null)}
          onSaved={(result) => {
            const kind = saveDraft;
            setSaveDraft(null);
            setCart(freshCart());
            if (kind === 'quote') setQuoteDocument(result.id);
            else searchRef.current?.focus();
          }}
        />
      )}
      {held && (
        <HeldSales
          actor={data.actor!}
          hasCart={cart.rows.length > 0}
          onClose={() => setHeld(false)}
          onLoad={(next) => {
            setCart(next);
            setHeld(false);
            toast.success('Venta recuperada. Revisa el ticket antes de cobrar.');
          }}
        />
      )}
      <Modal
        open={clear}
        onClose={() => setClear(false)}
        title="¿Vaciar el ticket actual?"
        description={
          fixedQuote
            ? 'La cotización se conserva y podrás recuperarla después.'
            : cart.heldSaleId
              ? 'La venta guardada se conserva con sus últimos datos guardados.'
              : 'Se quitarán estas partidas del carrito.'
        }
      >
        <div className="dialog-actions">
          <Button variant="outline" onClick={() => setClear(false)}>
            Conservar ticket
          </Button>
          <Button
            variant="destructive"
            onClick={() => {
              setCart(freshCart());
              setClear(false);
              searchRef.current?.focus();
            }}
          >
            Vaciar ticket
          </Button>
        </div>
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
            {lastSale.sale.tenderedCents > 0 && (
              <div className="change-banner">
                <span>Cambio para el cliente</span>
                <b>{money(lastSale.sale.changeCents)}</b>
              </div>
            )}
            <div className="success-actions">
              <Button variant="outline" onClick={() => setReceipt(true)}>
                <ReceiptText size={17} />
                Ver e imprimir ticket
              </Button>
              <Button
                onClick={() => {
                  setLastSale(null);
                  requestAnimationFrame(() => searchRef.current?.focus());
                }}
              >
                Siguiente venta
                <ArrowRight size={17} />
              </Button>
            </div>
          </>
        )}
      </Modal>
      {quoteDocument !== null && (
        <DocumentDialog
          data={data}
          request={{ kind: 'quote', id: quoteDocument }}
          onClose={() => {
            setQuoteDocument(null);
            requestAnimationFrame(() => searchRef.current?.focus());
          }}
        />
      )}
      {receipt && lastSale && (
        <DocumentDialog
          data={data}
          request={{ kind: 'sale', id: lastSale.sale.id }}
          onClose={() => {
            setReceipt(false);
            setLastSale(null);
            requestAnimationFrame(() => searchRef.current?.focus());
          }}
        />
      )}
    </div>
  );
}
