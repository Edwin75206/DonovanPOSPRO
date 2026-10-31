import { useState } from 'react';
import { Check, ShieldCheck } from 'lucide-react';
import { api, normalizeError, type Charge, type Checkout, type SaleDetail } from '@/lib/api';
import { money, parseScaled, quantity, today } from '@/lib/utils';
import { Button, Field, Failure, useOperation } from '@/components/common';
import type { Cart } from './Register';

export function PaymentForm({
  charge,
  cart,
  allowCredit,
  onBusy,
  onSuccess,
}: {
  charge: Charge;
  cart: Cart;
  allowCredit: boolean;
  onBusy: (busy: boolean) => void;
  onSuccess: (sale: SaleDetail) => void;
}) {
  const [method, setMethod] = useState(0);
  const [received, setReceived] = useState(String(charge.totalCents / 100));
  const [amounts, setAmounts] = useState([String(charge.totalCents / 100), '', '', '']);
  const [reference, setReference] = useState('');
  const [dueDate, setDueDate] = useState('');
  const op = useOperation();
  const creditAvailable = allowCredit && !!cart.customerId;
  const methods = ['Efectivo', 'Tarjeta', 'Transferencia', 'Crédito'];
  const payments =
    method === 4
      ? amounts
          .map((value, i) => ({
            method: i,
            amountCents: parseScaled(value),
            reference: i === 1 || i === 2 ? reference : '',
          }))
          .filter((p) => p.amountCents > 0)
      : [{ method, amountCents: charge.totalCents, reference }];
  if (!payments.length && charge.totalCents === 0)
    payments.push({ method: 0, amountCents: 0, reference: '' });
  const paid = payments.reduce((sum, p) => sum + p.amountCents, 0);
  const cash = payments.find((p) => p.method === 0)?.amountCents ?? 0;
  const credit = payments.find((p) => p.method === 3)?.amountCents ?? 0;
  const change = cash ? parseScaled(received) - cash : 0;
  const invalid =
    paid !== charge.totalCents ||
    change < 0 ||
    payments.some((p) => p.amountCents < 0) ||
    (credit > 0 && (!creditAvailable || !dueDate || dueDate < today()));
  return (
    <form
      className="payment-layout"
      onSubmit={async (e) => {
        e.preventDefault();
        if (op.busy || invalid) return;
        const request: Checkout = {
          requestId: cart.requestId,
          items: cart.rows.map((r) => ({
            productId: r.product.id,
            quantityUnits: r.quantityUnits,
            wholesale: r.wholesale,
          })),
          payments,
          tenderedCents: cash ? parseScaled(received) : 0,
          discountParts: cart.discountParts,
          customerId: cart.customerId,
          dueDate: credit ? dueDate : null,
          quoteId: cart.quoteId,
          heldSaleId: cart.heldSaleId,
        };
        onBusy(true);
        try {
          await op.run(async () => {
            try {
              return await api('checkout', request);
            } catch (error) {
              const failure = normalizeError(error);
              if (
                failure.retryable ||
                failure.code === 'REQUEST_CHANGED' ||
                failure.code === 'UNEXPECTED'
              ) {
                const saved = await api('checkoutResult', { requestId: request.requestId }).catch(
                  () => null,
                );
                if (saved) return saved;
              }
              throw error;
            }
          }, onSuccess);
        } finally {
          onBusy(false);
        }
      }}
    >
      <section className="payment-overview">
        <span className="eyebrow">TOTAL DE LA VENTA</span>
        <h2>{money(charge.totalCents)}</h2>
        <p>{cart.customerName || 'Público en general'} · IVA incluido</p>
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
          <span>Se guardan juntos el pago, la venta y el movimiento de inventario.</span>
        </div>
      </section>
      <section className="payment-fields">
        <fieldset disabled={op.busy}>
          <span className="field-label">¿Cómo paga el cliente?</span>
          <div className="payment-methods">
            {[0, 1, 2, ...(creditAvailable ? [3] : []), 4].map((value) => (
              <button
                type="button"
                className={method === value ? 'active' : ''}
                key={value}
                onClick={() => setMethod(value)}
              >
                {value === 4 ? 'Mixto' : methods[value]}
                {method === value && <Check size={14} />}
              </button>
            ))}
          </div>
          {method === 4 && (
            <div className="mixed-payments">
              {methods.map((name, i) =>
                i === 3 && !creditAvailable ? null : (
                  <Field label={`${name} (MXN)`} key={name}>
                    <div className="split-input">
                      <input
                        type="number"
                        min="0"
                        max={charge.totalCents / 100}
                        step="0.01"
                        value={amounts[i]}
                        placeholder="0.00"
                        onChange={(e) =>
                          setAmounts((a) => a.map((v, index) => (index === i ? e.target.value : v)))
                        }
                      />
                      <button
                        type="button"
                        title={`Completar lo restante con ${name.toLowerCase()}`}
                        onClick={() =>
                          setAmounts((a) =>
                            a.map((v, index) =>
                              index === i
                                ? String(
                                    Math.max(
                                      0,
                                      charge.totalCents -
                                        a.reduce(
                                          (sum, x, n) => sum + (n === i ? 0 : parseScaled(x)),
                                          0,
                                        ),
                                    ) / 100,
                                  )
                                : v,
                            ),
                          )
                        }
                      >
                        Restante
                      </button>
                    </div>
                  </Field>
                ),
              )}
              <div
                className={`payment-balance ${paid !== charge.totalCents ? 'amount-pending' : 'amount-positive'}`}
              >
                {paid === charge.totalCents
                  ? 'Importes completos'
                  : paid < charge.totalCents
                    ? `Por asignar: ${money(charge.totalCents - paid)}`
                    : `Exceso asignado: ${money(paid - charge.totalCents)}`}
              </div>
            </div>
          )}
          {cash > 0 && (
            <>
              <Field label="Efectivo recibido">
                <div className="money-input">
                  <span>$</span>
                  <input
                    autoFocus
                    type="number"
                    min={cash / 100}
                    step="0.01"
                    max="1000000000"
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
                onClick={() => setReceived(String(cash / 100))}
              >
                Efectivo exacto
              </button>
              <div className={`change-banner ${change < 0 ? 'negative' : ''}`}>
                <span>{change < 0 ? 'Falta por recibir' : 'Cambio para el cliente'}</span>
                <b>{money(Math.abs(change))}</b>
              </div>
            </>
          )}
          {payments.some((p) => p.method === 1 || p.method === 2) && (
            <Field label="Referencia de tarjeta / transferencia (opcional)">
              <input
                value={reference}
                maxLength={200}
                onChange={(e) => setReference(e.target.value)}
                placeholder="Folio de la operación"
              />
            </Field>
          )}
          {credit > 0 && (
            <Field
              label="Fecha de pago del crédito"
              hint={`Se cargarán ${money(credit)} a ${cart.customerName}. Validaremos el límite disponible al confirmar.`}
            >
              <input
                type="date"
                required
                min={today()}
                value={dueDate}
                onChange={(e) => setDueDate(e.target.value)}
              />
            </Field>
          )}
          {allowCredit && !cart.customerId && (
            <p className="muted">
              Para vender a crédito, vuelve al ticket y selecciona un cliente.
            </p>
          )}
        </fieldset>
        <Failure error={op.error} />
        <Button type="submit" className="wide" disabled={op.busy || invalid}>
          {op.busy ? 'Registrando venta…' : 'Confirmar pago'}
          <Check size={18} />
        </Button>
        <p className="payment-hint">
          La venta se guarda antes de imprimir. Reimprimir no genera otro cobro.
        </p>
      </section>
    </form>
  );
}
