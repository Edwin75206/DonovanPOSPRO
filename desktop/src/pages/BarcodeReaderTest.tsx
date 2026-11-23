import { useId, useRef, useState } from 'react';
import { ScanBarcode } from 'lucide-react';
import { Button, Failure, useOperation } from '@/components/common';
import { api } from '@/lib/api';
import { ScanBuffer } from '@/lib/register';

export function BarcodeReaderTest({ count }: { count: number | null }) {
  const fieldId = useId();
  const buffer = useRef(new ScanBuffer());
  const [input, setInput] = useState('');
  const [reading, setReading] = useState<{ code: string; product?: string } | null>(null);
  const lookup = useOperation();
  const check = (value: string) => {
    const code = value.trim();
    if (!code || lookup.busy) return;
    setReading({ code });
    setInput('');
    buffer.current.reset();
    void lookup.run(
      () => api('productForCode', { code }),
      (product) => setReading({ code, product: product.name }),
    );
  };

  return (
    <section className="surface printer-form barcode-reader-test">
      <h2>
        <ScanBarcode size={20} /> Lector de códigos de barras
      </h2>
      <p className="muted" role="status">
        {count === null
          ? 'Puedes comprobar el lector en este campo.'
          : count === 0
            ? 'El sistema no identifica un lector dedicado. Si el tuyo funciona como teclado USB, compruébalo aquí.'
            : `Windows identifica ${count} ${count === 1 ? 'lector POS' : 'lectores POS'}. Para escanear en caja, configura el modo teclado.`}
      </p>
      <div className="barcode-reader-input">
        <label htmlFor={fieldId}>Verificación de lectura</label>
        <div className="barcode-reader-controls">
          <input
            id={fieldId}
            aria-describedby={fieldId + '-hint'}
            value={input}
            maxLength={100}
            placeholder="Escanea un código de barras"
            onChange={(event) => setInput(event.target.value)}
            onBlur={() => buffer.current.reset()}
            onKeyDown={(event) => {
              const modified =
                event.ctrlKey ||
                event.altKey ||
                event.metaKey ||
                event.nativeEvent.isComposing ||
                event.repeat;
              const scanned = buffer.current.key(event.key, performance.now(), true, modified);
              if (!modified && (scanned || event.key === 'Enter')) {
                event.preventDefault();
                check(scanned ?? event.currentTarget.value);
              }
            }}
          />
          <Button
            type="button"
            variant="outline"
            disabled={!input.trim() || lookup.busy}
            onClick={() => check(input)}
          >
            Comprobar código
          </Button>
        </div>
        <small id={fieldId + '-hint'}>
          Enfoca este campo y escanea para consultar el producto.
        </small>
      </div>
      {reading && (
        <div className="printer-status" role="status">
          <strong>Código recibido · {reading.code}</strong>
          <span>{lookup.busy ? 'Buscando en el catálogo…' : reading.product}</span>
        </div>
      )}
      <Failure error={lookup.error} />
    </section>
  );
}
