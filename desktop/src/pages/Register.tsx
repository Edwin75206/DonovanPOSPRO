import { PaymentForm } from './PaymentForm';
import { useCallback, useEffect, useRef, useState } from 'react';
import { motion } from 'motion/react';
import {
  Search,
  ScanBarcode,
  Trash2,
  ArrowRight,
  Check,
  ReceiptText,
  Tag,
  Pause,
  History,
  UserRound,
  X,
} from 'lucide-react';
import {
  api,
  can,
  normalizeError,
  type Bootstrap,
  type Product,
  type Charge,
  type SaleDetail,
  type CashOverview,
} from '@/lib/api';
import { Button, Modal, Field, Failure, reportError } from '@/components/common';
import { money, quantity, units, parseScaled } from '@/lib/utils';
import { DocumentDialog, autoPrintSale } from '@/components/DocumentViewer';
import { toast } from 'sonner';
import { CustomerPicker, SaveDraft, HeldSales } from './CartActions';
import { useVisual } from '@/lib/appearance';
import { RegisterCash, RegisterRecent } from './RegisterTools';
import { addQuantity, previewAmounts, ScanBuffer, validQuantity } from '@/lib/register';
import { ProductPicker } from './ProductPicker';
import { RegisterLines } from './RegisterLines';
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
export function Register({
  data,
  cart,
  setCart,
  onCash,
  onSaved,
  onHistory,
  onQuotes,
  searchRequest = 0,
}: {
  data: Bootstrap;
  cart: Cart;
  setCart: (value: Cart | ((v: Cart) => Cart)) => void;
  onCash: () => void;
  onSaved: () => void;
  onHistory: () => void;
  onQuotes: () => void;
  searchRequest?: number;
}) {
  const [codeText, setCodeText] = useState('');
  const [picker, setPicker] = useState(false);
  const [scanError, setScanError] = useState('');
  const scanPending = useRef(0);
  const checkoutStarting = useRef(false);
  const previousSearchRequest = useRef(0);
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
  const [checkoutBusy, setCheckoutBusy] = useState(false);
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
  const priceId = useRef(0);
  const [error, setError] = useState<ReturnType<typeof reportError> | null>(null);
  const items = cart.rows.map((r) => ({
    productId: r.product.id,
    quantityUnits: r.quantityUnits,
    wholesale: r.wholesale,
  }));
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
        return false;
      }
      const existing = cartRef.current.rows.find((x) => x.product.id === product.id);
      const next = addQuantity(product, existing?.quantityUnits);
      if (!product.active || next === null) {
        toast.error(
          product.stockUnits <= 0
            ? `${product.name}: agotado.`
            : `Solo hay ${quantity(product.stockUnits)} ${units[product.unit]} disponibles; revisa tu ticket.`,
        );
        return false;
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
      setLastSale(null);
      setScanError('');
      setAddedMessage(`Agregado: ${product.name}`);
      setHighlight(product.id);
      setTimeout(() => setHighlight((current) => (current === product.id ? null : current)), 420);
      const active = document.activeElement;
      if (
        active === searchRef.current ||
        !active?.matches('input,textarea,select,[contenteditable=true]')
      )
        searchRef.current?.focus();
      return true;
    },
    [updateCart, fixedQuote],
  );
  const scan = useCallback(
    (code: string) => {
      if (!code.trim() || checkoutStarting.current) return;
      setCodeText('');
      setScanError('');
      scanPending.current++;
      setScanBusy((n) => n + 1);
      scanQueue.current = scanQueue.current
        .then(async () => {
          if (!mounted.current || document.querySelector('[role=dialog]')) return;
          try {
            const found = await api('productForCode', { code: code.trim() });
            if (!mounted.current || document.querySelector('[role=dialog]')) return;
            add(found);
          } catch (e) {
            if (mounted.current) setScanError(normalizeError(e).message);
          }
        })
        .finally(() => {
          scanPending.current--;
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
      scanPending.current ||
      checkoutStarting.current ||
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
    const snapshot = cartRef.current;
    checkoutStarting.current = true;
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
      if (!mounted.current || snapshot !== cartRef.current) {
        if (mounted.current)
          toast.info('La venta cambió. Revisa las cantidades y vuelve a cobrar.');
        return;
      }
      setCharge(current);
      setCheckout(true);
    } catch (e) {
      reportError(e);
    } finally {
      checkoutStarting.current = false;
      setPricing(false);
    }
  }, [cart, cash, checkout, pricing, scanBusy, invalidRows, onCash]);
  const openPicker = useCallback(() => {
    if (fixedQuote || scanPending.current || checkoutStarting.current) return;
    scanner.current.reset();
    setPicker(true);
  }, [fixedQuote]);
  useEffect(() => {
    if (searchRequest && previousSearchRequest.current !== searchRequest) {
      previousSearchRequest.current = searchRequest;
      openPicker();
    }
  }, [searchRequest, openPicker]);
  useEffect(() => {
    if (!addedMessage) return;
    const timer = setTimeout(() => setAddedMessage(''), 2400);
    return () => clearTimeout(timer);
  }, [addedMessage, highlight]);
  useEffect(() => {
    setInvalidRows(new Set());
    setDiscount(false);
    setCodeText('');
    setScanError('');
    scanner.current.reset();
  }, [cart.requestId]);
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
      if (event.key === 'F2') {
        event.preventDefault();
        event.stopPropagation();
        openPicker();
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
      const allowed =
        !checkout &&
        !receipt &&
        !checkoutStarting.current &&
        (!editable || event.target === searchRef.current);
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
  }, [beginCheckout, scan, checkout, receipt, openPicker]);
  const changeQty = (id: number, value: number) => {
    const product = cartRef.current.rows.find((r) => r.product.id === id)?.product;
    if (!product || !validQuantity(product, value)) return;
    setAddedMessage('');
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
    <div className="counter-register" data-tour="register">
      <header className="counter-heading">
        <div>
          <h1>Venta de mostrador</h1>
          <p>
            {data.actor?.name}
            <span>·</span>
            <span className="counter-reference">
              Ref. {cart.requestId.slice(0, 6).toUpperCase()}
            </span>
          </p>
        </div>
        <div className="counter-context">
          <button
            className={'counter-cash ' + (cash?.open ? 'is-open' : 'is-closed')}
            onClick={() => setQuickCash(true)}
            disabled={!cash}
          >
            <span />
            {cash?.open ? 'Caja abierta' : cash ? 'Abrir caja' : 'Consultando caja…'}
          </button>
          <button
            className="counter-customer"
            disabled={fixedQuote}
            onClick={() => setCustomerPicker(true)}
            data-tour="customer"
          >
            <UserRound size={17} />
            <span>{cart.customerName || 'Público en general'}</span>
            <span className="customer-change">Cambiar</span>
          </button>
        </div>
      </header>
      <div className="counter-capture" data-tour="product-search">
        <div className="code-input">
          <ScanBarcode size={22} />
          <input
            ref={searchRef}
            autoComplete="off"
            value={codeText}
            disabled={fixedQuote}
            aria-label="Capturar código de producto"
            placeholder="Escanea un código de barras…"
            maxLength={100}
            onChange={(e) => setCodeText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && codeText.trim()) {
                e.preventDefault();
                scan(codeText);
              }
            }}
          />
          <kbd>Enter</kbd>
        </div>
        <Button
          className="find-product-button"
          onClick={openPicker}
          disabled={fixedQuote || scanBusy > 0}
        >
          <Search size={19} />
          Buscar producto <kbd>F2</kbd>
        </Button>
      </div>
      <div
        className={'counter-status ' + (scanError ? 'has-code-error' : '')}
        role="status"
        aria-live="polite"
      >
        {scanBusy ? (
          <span>Identificando código…</span>
        ) : scanError ? (
          <>
            <span>{scanError}</span>
            <button onClick={openPicker}>Buscar con F2</button>
          </>
        ) : addedMessage ? (
          <span className="added-confirmation">
            <Check size={15} />
            {addedMessage}
          </span>
        ) : (
          <span>
            <ScanBarcode size={14} />
            Lector listo · Enter o Tab al terminar el código
          </span>
        )}
        <span>
          {cart.rows.length} {cart.rows.length === 1 ? 'artículo' : 'artículos'} en la venta
        </span>
      </div>
      {fixedQuote && (
        <p className="quote-lock">{cart.sourceLabel} · Precios y cantidades acordados.</p>
      )}
      {cart.editingQuote && (
        <p className="quote-lock">
          Editando {cart.sourceLabel} · Guarda los cambios antes de vender.
        </p>
      )}
      <RegisterLines
        cart={cart}
        charge={charge}
        lineCents={preview?.lineCents}
        locked={fixedQuote}
        canDiscount={can(data.actor!, 'discount')}
        highlight={highlight}
        invalidRows={invalidRows}
        onQuantity={changeQty}
        onCapture={() => searchRef.current?.focus()}
        onValidity={(id, valid) =>
          setInvalidRows((current) => {
            const next = new Set(current);
            if (valid) next.delete(id);
            else next.add(id);
            return next;
          })
        }
        onRemove={(id) => {
          updateCart((c) => ({ ...c, rows: c.rows.filter((r) => r.product.id !== id) }));
          setInvalidRows((current) => {
            const next = new Set(current);
            next.delete(id);
            return next;
          });
          setAddedMessage('');
          searchRef.current?.focus();
        }}
        onWholesale={(id) =>
          updateCart((c) => ({
            ...c,
            rows: c.rows.map((r) => (r.product.id === id ? { ...r, wholesale: !r.wholesale } : r)),
          }))
        }
      />
      <Failure error={error} />
      {lastSale && (
        <motion.div
          className="counter-sale-confirmation"
          role="status"
          initial={{ opacity: 0, y: 5 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: visual.duration }}
        >
          <span className="confirmation-check">
            <Check size={20} />
          </span>
          <div>
            <strong>Venta registrada · {lastSale.sale.folio}</strong>
            <span>{money(lastSale.sale.totalCents)} · Lista para la siguiente venta</span>
          </div>
          {lastSale.sale.tenderedCents > 0 && (
            <div className="confirmation-change">
              <span>Cambio a entregar</span>
              <strong>{money(lastSale.sale.changeCents)}</strong>
            </div>
          )}
          <Button variant="outline" onClick={() => setReceipt(true)}>
            <ReceiptText size={17} />
            Ver e imprimir ticket
          </Button>
          <button
            className="icon-button"
            aria-label="Ocultar confirmación de venta"
            onClick={() => setLastSale(null)}
          >
            <X size={16} />
          </button>
        </motion.div>
      )}
      <footer className="counter-summary" data-tour="cart">
        <div className="counter-tools" data-tour="cart-tools">
          <div>
            <button
              disabled={!cart.rows.length || fixedQuote || pendingQuantity || !!scanBusy || pricing}
              onClick={() => setSaveDraft('held')}
            >
              <Pause size={15} />
              Poner en espera
            </button>
            <button
              disabled={!cart.rows.length || fixedQuote || pendingQuantity || !!scanBusy || pricing}
              onClick={() => setSaveDraft('quote')}
            >
              <ReceiptText size={15} />
              Cotizar
            </button>
            <button onClick={() => setHeld(true)}>
              <History size={15} />
              En espera
            </button>
            <button onClick={() => setRecent(true)}>Ventas recientes</button>
            <button onClick={onQuotes}>Cotizaciones</button>
          </div>
          <button
            className="clear-sale"
            disabled={!cart.rows.length || !!scanBusy}
            onClick={() => setClear(true)}
          >
            <Trash2 size={14} />
            Vaciar
          </button>
        </div>
        <div className="counter-totals">
          <div className="counter-breakdown">
            <dl>
              <div>
                <dt>Subtotal sin IVA</dt>
                <dd>
                  {charge ? money(total - charge.taxCents) : cart.rows.length ? '…' : money(0)}
                </dd>
              </div>
              <div>
                <dt>IVA incluido</dt>
                <dd>{charge ? money(charge.taxCents) : cart.rows.length ? '…' : money(0)}</dd>
              </div>
              {!!cart.discountParts && (
                <div>
                  <dt>Descuento incluido</dt>
                  <dd>−{money(charge?.discountCents ?? preview?.discountCents ?? 0)}</dd>
                </div>
              )}
            </dl>
            {can(data.actor!, 'discount') && !fixedQuote && (
              <button className="counter-discount" onClick={() => setDiscount(!discount)}>
                <Tag size={13} />
                {discount ? 'Cerrar descuento' : 'Aplicar descuento'}
              </button>
            )}
            {discount && (
              <Field label="Descuento sobre el total (%)">
                <input
                  type="number"
                  min={0}
                  max={100}
                  step={0.01}
                  value={cart.discountParts / 10000}
                  onChange={(e) =>
                    updateCart((c) => ({ ...c, discountParts: parseScaled(e.target.value, 10000) }))
                  }
                />
              </Field>
            )}
          </div>
          <div className="counter-total">
            <span>
              Total a pagar <small>MXN</small>
            </span>
            <motion.strong
              key={total}
              initial={{ opacity: 0.7 }}
              animate={{ opacity: 1 }}
              transition={{ duration: visual.duration }}
            >
              {charge || preview ? money(total) : cart.rows.length ? '…' : money(0)}
            </motion.strong>
          </div>
          <Button
            className="counter-checkout"
            data-tour="checkout"
            disabled={!cart.rows.length || pricing || !!scanBusy || pendingQuantity || !!error}
            onClick={() => void beginCheckout()}
          >
            {cart.editingQuote ? 'Guardar cotización' : 'Cobrar'}
            <kbd>F8</kbd>
            <ArrowRight size={20} />
          </Button>
        </div>
      </footer>
      {picker && (
        <ProductPicker
          rows={cart.rows}
          onSelect={add}
          onClose={() => setPicker(false)}
          returnFocusRef={searchRef}
        />
      )}
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
        returnFocusRef={searchRef}
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
              updateCart(() => freshCart());
              autoPrintSale(data, sale.sale.id);
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
        returnFocusRef={searchRef}
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
