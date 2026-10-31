import { useEffect, useRef, useState } from 'react';
import { motion } from 'motion/react';
import { ScanBarcode, SlidersHorizontal, Trash2 } from 'lucide-react';
import type { Charge } from '@/lib/api';
import { money, quantity, units } from '@/lib/utils';
import { useVisual } from '@/lib/appearance';
import { QuantityEditor } from '@/components/QuantityEditor';
import { StockIndicator } from '@/components/StockIndicator';
import type { Cart } from './Register';

export function RegisterLines({
  cart,
  charge,
  lineCents,
  locked,
  canDiscount,
  highlight,
  invalidRows,
  onQuantity,
  onValidity,
  onRemove,
  onWholesale,
  onCapture,
}: {
  cart: Cart;
  charge: Charge | null;
  lineCents?: number[];
  locked: boolean;
  canDiscount: boolean;
  highlight: number | null;
  invalidRows: Set<number>;
  onQuantity: (id: number, value: number) => void;
  onValidity: (id: number, valid: boolean) => void;
  onRemove: (id: number) => void;
  onWholesale: (id: number) => void;
  onCapture: () => void;
}) {
  const [selected, setSelected] = useState<number | null>(null);
  const container = useRef<HTMLDivElement>(null);
  const visual = useVisual();
  useEffect(() => {
    if (highlight !== null) {
      setSelected(highlight);
      container.current
        ?.querySelector(`[data-product-id="${highlight}"]`)
        ?.scrollIntoView?.({ block: 'nearest' });
    }
  }, [highlight, cart.rows]);
  return (
    <div
      className={'counter-lines' + (!cart.rows.length ? ' is-empty' : '')}
      ref={container}
      tabIndex={0}
      role="region"
      aria-label="Artículos de la venta"
      onKeyDown={(e) => {
        if ((e.target as HTMLElement).closest('input,button,select')) return;
        if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
          e.preventDefault();
          const index = cart.rows.findIndex((r) => r.product.id === selected);
          const next =
            cart.rows[
              Math.max(0, Math.min(cart.rows.length - 1, index + (e.key === 'ArrowDown' ? 1 : -1)))
            ];
          if (next) {
            setSelected(next.product.id);
            container.current
              ?.querySelector(`[data-product-id="${next.product.id}"]`)
              ?.scrollIntoView?.({ block: 'nearest' });
          }
        }
        if (e.key === 'Enter') {
          e.preventDefault();
          container.current
            ?.querySelector<HTMLInputElement>(`[data-product-id="${selected}"] input`)
            ?.focus();
        }
        if (e.key === 'Delete' && selected !== null && !locked) {
          e.preventDefault();
          onRemove(selected);
        }
        if (e.key === 'Escape') onCapture();
      }}
    >
      <table className="sale-table" aria-label="Venta actual">
        <thead>
          <tr>
            <th className="quantity-heading">Cantidad</th>
            <th>Unidad</th>
            <th>Descripción</th>
            <th className="numeric">Precio unitario</th>
            {cart.discountParts > 0 && <th className="numeric">Desc.</th>}
            <th className="numeric">Importe</th>
            <th>
              <span className="sr-only">Retirar</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {cart.rows.map((row, index) => {
            const line = charge?.lines.find((l) => l.productId === row.product.id);
            const amount = line?.totalCents ?? lineCents?.[index];
            return (
              <motion.tr
                key={row.product.id}
                data-product-id={row.product.id}
                aria-selected={selected === row.product.id}
                className={`${highlight === row.product.id ? 'just-added' : ''} ${invalidRows.has(row.product.id) ? 'invalid-quantity' : ''}`}
                initial={{ opacity: 0, x: 6 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ duration: visual.duration }}
                onClick={() => setSelected(row.product.id)}
                onFocus={() => setSelected(row.product.id)}
              >
                <td>
                  <QuantityEditor
                    product={row.product}
                    value={row.quantityUnits}
                    disabled={locked}
                    onChange={(value) => onQuantity(row.product.id, value)}
                    onValidity={(valid) => onValidity(row.product.id, valid)}
                    onDone={onCapture}
                  />
                </td>
                <td className="unit-cell">{units[row.product.unit]}</td>
                <td className="description-cell">
                  <strong title={row.product.name}>{row.product.name}</strong>
                  <div className="sale-line-meta">
                    <span>{row.product.sku}</span>
                    <StockIndicator product={row.product} reserved={row.quantityUnits} />
                  </div>
                  {invalidRows.has(row.product.id) && (
                    <small className="quantity-warning">
                      Cantidad no válida. Máximo: {quantity(row.product.stockUnits)}{' '}
                      {units[row.product.unit]}.
                    </small>
                  )}
                </td>
                <td className="numeric">
                  <span>
                    {money(
                      line?.priceCents ??
                        (row.wholesale ? row.product.wholesaleCents : row.product.priceCents),
                    )}
                  </span>
                  {canDiscount && !locked && (
                    <button
                      className="price-mode"
                      onClick={() => onWholesale(row.product.id)}
                      aria-label={`Cambiar tarifa de ${row.product.name}`}
                    >
                      {row.wholesale ? 'Mayoreo' : 'Menudeo'}
                      <SlidersHorizontal size={11} />
                    </button>
                  )}
                </td>
                {cart.discountParts > 0 && (
                  <td className="numeric discount-cell">{quantity(cart.discountParts)}%</td>
                )}
                <td className="numeric line-amount">
                  <b>{amount === undefined ? '…' : money(amount)}</b>
                </td>
                <td>
                  <button
                    className="icon-button remove-line"
                    disabled={locked}
                    aria-label={`Quitar ${row.product.name}`}
                    onClick={() => onRemove(row.product.id)}
                  >
                    <Trash2 size={16} />
                  </button>
                </td>
              </motion.tr>
            );
          })}
        </tbody>
      </table>
      {!cart.rows.length && (
        <div className="counter-empty">
          <div className="counter-empty-icon">
            <ScanBarcode size={34} strokeWidth={1.4} />
          </div>
          <h2>Lista para tu siguiente cliente</h2>
          <p>
            Escanea un código para comenzar
            <br />o presiona <kbd>F2</kbd> para buscar un producto.
          </p>
          <span>Los artículos aparecerán aquí.</span>
        </div>
      )}
    </div>
  );
}
