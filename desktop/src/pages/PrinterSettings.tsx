import { useCallback, useEffect, useState } from 'react';
import { Printer, RefreshCw, Save, Eye, FileDown, ReceiptText } from 'lucide-react';
import { api, type Bootstrap, type TicketConfig } from '@/lib/api';
import { useResource } from '@/lib/useResource';
import { Button, Busy, Failure, Field, PageTitle, useOperation } from '@/components/common';
import { PaperPreview, queueDocument } from '@/components/DocumentViewer';
import { toast } from 'sonner';
import { BarcodeReaderTest } from './BarcodeReaderTest';

export function PrinterSettings({
  data,
  onSaved,
  onDirty,
}: {
  data: Bootstrap;
  onSaved: (data: Bootstrap) => void;
  onDirty: (dirty: boolean) => void;
}) {
  const source = useResource(() => api('ticketConfig'), [data.settings]);
  if (source.busy) return <Busy>Consultando la configuración…</Busy>;
  if (!source.data) return <Failure error={source.error} retry={source.reload} />;
  return <PrinterEditor source={source.data} data={data} onSaved={onSaved} onDirty={onDirty} />;
}
function PrinterEditor({
  source,
  data,
  onSaved,
  onDirty,
}: {
  source: TicketConfig;
  data: Bootstrap;
  onSaved: (data: Bootstrap) => void;
  onDirty: (dirty: boolean) => void;
}) {
  const [draft, setDraft] = useState(source);
  const [previewDraft, setPreviewDraft] = useState(source);
  const [printing, setPrinting] = useState(false);
  const printers = useResource(() => api('printers'), []);
  const peripherals = useResource(() => api('peripheralStatus'), []);
  const usbPrinters = peripherals.data?.usbPrinters ?? null;
  const selected = printers.data?.find((p) => p.name === draft.printerName);
  const status = useResource(
    () => (selected ? api('printerDetails', { printer: selected.name }) : Promise.resolve(null)),
    [selected?.name],
  );
  const details = status.data?.name === selected?.name ? status.data : null;
  const { reload: reloadPrinters } = printers;
  const { reload: reloadPeripherals } = peripherals;
  const { reload: reloadStatus } = status;
  const checking = printers.busy || peripherals.busy || status.busy;
  const refreshDevices = useCallback(() => {
    reloadPrinters();
    reloadPeripherals();
    reloadStatus();
  }, [reloadPrinters, reloadPeripherals, reloadStatus]);
  const preview = useResource(
    () => api('document', { kind: 'ticketTest', id: 0, ticket: previewDraft }),
    [JSON.stringify(previewDraft)],
  );
  const op = useOperation();
  const exportOp = useOperation();
  const dirty = JSON.stringify(draft) !== JSON.stringify(source);
  useEffect(() => {
    if (checking) return;
    const timer = window.setTimeout(refreshDevices, 4000);
    return () => window.clearTimeout(timer);
  }, [checking, refreshDevices]);
  useEffect(() => {
    onDirty(dirty);
    return () => onDirty(false);
  }, [dirty, onDirty]);
  useEffect(() => {
    const timer = setTimeout(() => setPreviewDraft(draft), 250);
    return () => clearTimeout(timer);
  }, [draft]);
  const patch = (value: Partial<TicketConfig>) => setDraft((current) => ({ ...current, ...value }));
  return (
    <main className="page-scroll page-container printer-settings">
      <PageTitle
        title="Periféricos"
        description="Conecta la impresora o el lector. La detección se actualiza mientras esta pantalla esté abierta."
        actions={
          <Button variant="outline" disabled={checking} onClick={refreshDevices}>
            <RefreshCw size={15} />
            Detectar ahora
          </Button>
        }
      />
      <Failure error={peripherals.error} retry={reloadPeripherals} />
      <BarcodeReaderTest count={peripherals.data?.barcodeReaders ?? null} />
      <div className="printer-columns">
        <form
          className="surface printer-form"
          onSubmit={(e) => {
            e.preventDefault();
            void op.run(
              () => api('saveTicketConfig', draft),
              (next) => {
                toast.success('Impresora y ticket configurados.');
                onSaved(next);
              },
            );
          }}
        >
          <h2>
            <Printer size={20} /> Tu impresora de tickets
          </h2>
          {usbPrinters !== null && (
            <p className="muted" role="status">
              {usbPrinters === 0
                ? 'Windows no detecta impresoras USB conectadas.'
                : `Windows detecta ${usbPrinters} ${usbPrinters === 1 ? 'impresora USB conectada' : 'impresoras USB conectadas'}.`}
            </p>
          )}
          <Field label="Impresora instalada">
            <select
              value={draft.printerName}
              onChange={(e) => patch({ printerName: e.target.value, autoPrint: false })}
            >
              <option value="">Sin impresora · vista previa y PDF</option>
              {draft.printerName && !selected && (
                <option value={draft.printerName}>{draft.printerName} · No disponible</option>
              )}
              {printers.data?.map((p) => (
                <option key={p.name} value={p.name}>
                  {p.name}
                  {p.isDefault ? ' · Predeterminada del sistema' : ''}
                </option>
              ))}
            </select>
          </Field>
          {printers.data && (
            <p className="muted" role="status">
              {printers.data.some((p) => !p.virtualPrinter)
                ? 'Hay impresoras físicas instaladas. Elige la ticketera para consultar su estado.'
                : 'Sin impresoras físicas instaladas. Conecta la ticketera e instala su controlador si el sistema lo pide.'}
            </p>
          )}
          <Failure error={printers.error} retry={printers.reload} />
          <Failure error={status.error} retry={status.reload} />
          {details && (
            <div className={`printer-status ${details.offline ? 'offline' : ''}`} role="status">
              <strong>{details.status}</strong>
              <span>
                {details.driver} · {details.queuedJobs} trabajos en cola
              </span>
              <small>
                El estado depende del controlador; no confirma que el papel haya salido.
              </small>
            </div>
          )}
          <div className="form-grid">
            <Field label="Ancho de papel">
              <select
                value={draft.widthMm}
                onChange={(e) => patch({ widthMm: Number(e.target.value) })}
              >
                <option value={58}>Ticket de 58 mm</option>
                <option value={80}>Ticket de 80 mm</option>
              </select>
            </Field>
            <Field label="Márgenes (mm)">
              <input
                type="number"
                required
                min="2"
                max="8"
                step="1"
                value={draft.marginMm}
                onChange={(e) => patch({ marginMm: Number(e.target.value) })}
              />
            </Field>
          </div>
          {details && (
            <p className="muted">
              {details.paperWidthsMm.some((w) => Math.abs(w - draft.widthMm) <= 1)
                ? 'El controlador declara este ancho de papel.'
                : 'Comprueba este formato en las propiedades del controlador. Algunos dispositivos requieren crear un tamaño personalizado.'}
            </p>
          )}
          <Field label="Copias por impresión">
            <select
              value={draft.copies}
              onChange={(e) => patch({ copies: Number(e.target.value) })}
            >
              {[1, 2, 3].map((n) => (
                <option key={n} value={n}>
                  {n} {n === 1 ? 'copia' : 'copias'}
                </option>
              ))}
            </select>
          </Field>
          <label className="check-label">
            <input
              type="checkbox"
              checked={draft.autoPrint}
              disabled={!selected || selected.virtualPrinter}
              onChange={(e) => patch({ autoPrint: e.target.checked })}
            />
            Imprimir automáticamente después de cobrar
          </label>
          {!selected && (
            <p className="muted">Puedes seguir vendiendo, ver el ticket y guardarlo como PDF.</p>
          )}
          <h2>
            <ReceiptText size={20} /> Formato del comprobante
          </h2>
          <label className="check-label">
            <input
              type="checkbox"
              checked={draft.showLogo}
              onChange={(e) => patch({ showLogo: e.target.checked })}
            />
            Mostrar logotipo del negocio
          </label>
          <label className="check-label">
            <input
              type="checkbox"
              checked={draft.monochrome}
              onChange={(e) => patch({ monochrome: e.target.checked })}
            />
            Logo en blanco y negro para papel térmico
          </label>
          <Field label="Mensaje de agradecimiento">
            <textarea
              rows={2}
              maxLength={1500}
              value={draft.footer}
              onChange={(e) => patch({ footer: e.target.value })}
            />
          </Field>
          <Field label="Leyenda informativa">
            <textarea
              rows={2}
              maxLength={500}
              value={draft.legend}
              onChange={(e) => patch({ legend: e.target.value })}
            />
          </Field>
          <p className="muted">
            El nombre, domicilio, teléfono y logo se editan en Datos del negocio. El original del
            logo se conserva. Este comprobante no es un CFDI.
          </p>
          <Failure error={op.error} />
          <div className="dialog-actions">
            <Button
              type="button"
              variant="outline"
              disabled={!dirty || op.busy}
              onClick={() => setDraft(source)}
            >
              Restablecer cambios
            </Button>
            <Button type="submit" disabled={!dirty || op.busy}>
              <Save size={16} />
              {op.busy ? 'Guardando…' : 'Guardar configuración'}
            </Button>
          </div>
        </form>
        <section className="surface printer-preview">
          <h2>
            <Eye size={19} /> Así lo recibe tu cliente
          </h2>
          <p className="muted">
            Vista previa · {draft.widthMm} mm · {draft.copies}{' '}
            {draft.copies === 1 ? 'copia' : 'copias'} al imprimir
          </p>
          {preview.data ? (
            <PaperPreview document={preview.data} />
          ) : preview.busy ? (
            <Busy />
          ) : (
            <Failure error={preview.error} retry={preview.reload} />
          )}
          <div className="printer-test-actions">
            <Button
              variant="outline"
              disabled={
                printing || !selected || selected.virtualPrinter || preview.busy || !!preview.error
              }
              onClick={async () => {
                setPrinting(true);
                try {
                  await queueDocument(
                    { kind: 'ticketTest', id: 0, ticket: draft },
                    draft.printerName,
                    data.databaseId ?? '',
                  );
                } catch {
                } finally {
                  setPrinting(false);
                }
              }}
            >
              <Printer size={16} />
              {printing ? 'Enviando…' : 'Verificar impresión'}
            </Button>
            <Button
              variant="outline"
              disabled={exportOp.busy || preview.busy || !!preview.error}
              onClick={() =>
                void exportOp.run(
                  () => api('exportDocument', { kind: 'ticketTest', id: 0, ticket: draft }),
                  (result) => {
                    if (result)
                      toast.success('Comprobante de verificación guardado', { description: result.path });
                  },
                )
              }
            >
              <FileDown size={16} />
              Guardar muestra en PDF
            </Button>
          </div>
          <Failure error={exportOp.error} />
          <p className="muted">
            La verificación usa un comprobante de muestra. Comprueba el papel antes de activar la
            impresión automática.
          </p>
        </section>
      </div>
    </main>
  );
}
