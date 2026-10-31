import { useEffect, useRef, useState } from 'react';
import { Minus, Plus } from 'lucide-react';
import { toast } from 'sonner';
import type { Product } from '@/lib/api';
import { addQuantity, ScanBuffer, validQuantity } from '@/lib/register';
import { parseScaled, quantity, units } from '@/lib/utils';

export function QuantityEditor({
  product,
  value,
  disabled,
  onChange,
  onValidity,
  onDone,
}: {
  product: Product;
  value: number;
  disabled: boolean;
  onChange: (value: number) => void;
  onValidity: (valid: boolean) => void;
  onDone?: () => void;
}) {
  const [text, setText] = useState(String(value / 10000));
  const scanShield = useRef(new ScanBuffer());
  const beforeBurst = useRef(value);
  const previousKey = useRef(0);
  useEffect(
    () =>
      setText((current) =>
        parseScaled(current.replace(',', '.'), 10000) === value ? current : String(value / 10000),
      ),
    [value],
  );
  const integer = [0, 4, 5].includes(product.unit);
  const next = addQuantity(product, value);
  const commit = (raw: string) => {
    setText(raw);
    const digits = integer ? /^\d+$/ : /^\d+(?:[.,]\d{0,4})?$/;
    const parsed = parseScaled(raw.replace(',', '.'), 10000);
    const valid = digits.test(raw) && validQuantity(product, parsed);
    onValidity(valid);
    if (valid) onChange(parsed);
  };
  const reset = () => {
    if (parseScaled(text.replace(',', '.'), 10000) !== value || !text) {
      toast.info(
        `Cantidad conservada: ${quantity(value)} ${units[product.unit]}. Disponible: ${quantity(product.stockUnits)}.`,
      );
    }
    setText(String(value / 10000));
    onValidity(true);
  };
  return (
    <div className="quantity-control">
      <button
        type="button"
        aria-label={`Restar ${product.name}`}
        disabled={disabled || value <= (integer ? 10000 : 1)}
        onClick={() => {
          onValidity(true);
          onChange(Math.max(integer ? 10000 : 1, value - 10000));
        }}
      >
        <Minus size={14} />
      </button>
      <input
        type="text"
        inputMode={integer ? 'numeric' : 'decimal'}
        disabled={disabled}
        aria-label={`Cantidad de ${product.name}`}
        value={text}
        onFocus={(e) => {
          scanShield.current.reset();
          beforeBurst.current = value;
          previousKey.current = 0;
          e.currentTarget.select();
        }}
        onChange={(e) => commit(e.target.value)}
        onBlur={reset}
        onKeyDown={(e) => {
          const now = performance.now();
          if (now - previousKey.current > 70 || !previousKey.current) beforeBurst.current = value;
          previousKey.current = now;
          const code = scanShield.current.key(
            e.key,
            now,
            true,
            e.ctrlKey || e.altKey || e.metaKey || e.nativeEvent.isComposing || e.repeat,
          );
          if (code) {
            e.preventDefault();
            e.stopPropagation();
            onChange(beforeBurst.current);
            setText(String(beforeBurst.current / 10000));
            onValidity(true);
            toast.info('Lectura pausada al editar cantidades. La cantidad anterior se conserva.');
            return;
          }
          if (e.key === 'Enter' || e.key === 'Escape') {
            e.preventDefault();
            e.currentTarget.blur();
            onDone?.();
          }
        }}
      />
      <button
        type="button"
        aria-label={`Sumar ${product.name}`}
        disabled={disabled || next === null}
        onClick={() => {
          if (next !== null) {
            onValidity(true);
            onChange(next);
          }
        }}
      >
        <Plus size={14} />
      </button>
    </div>
  );
}
