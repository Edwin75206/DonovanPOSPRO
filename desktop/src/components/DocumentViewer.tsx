import { useEffect, useRef, useState } from 'react';
import { FileDown, Printer, Check, ArrowRight, LoaderCircle, RefreshCw } from 'lucide-react';
import { toast } from 'sonner';
import {
  api,
  type Bootstrap,
  type BusinessDocument,
  type DocumentRequest,
  type PrintJob,
  type UiFailure,
  normalizeError,
} from '@/lib/api';
import { useResource } from '@/lib/useResource';
import { Button, Busy, Failure, Field, Modal, useOperation } from './common';

const pending = new Map<string, Promise<PrintJob>>();
export function queueDocument(document: DocumentRequest, printer: string, databaseId: string) {
  const key = JSON.stringify([databaseId, document, printer]);
  const existing = pending.get(key);
  if (existing) return existing;
  const toastId = toast.loading('Enviando documento a Windows…', {
    description: 'Puedes continuar trabajando.',
  });
  const job = api('printDocument', { document, printer });
  pending.set(key, job);
  void job
    .then(
      (result) =>
        toast.success('Documento recibido por Windows', {
          id: toastId,
          description: `${result.printer} · Trabajo ${result.jobId}. Revisa la cola si no sale en papel.`,
          duration: 6000,
        }),
      (error) => {
        const failure = normalizeError(error);
        toast.error(failure.message, {
          id: toastId,
          description: `Referencia ${failure.incidentId} · La venta se conserva.`,
          duration: 9000,
        });
      },
    )
    .finally(() => pending.delete(key));
  return job;
}
export function autoPrintSale(data: Bootstrap, id: number) {
  if (data.settings?.autoPrint && data.settings.printerName)
    void queueDocument(
      { kind: 'sale', id, widthMm: data.settings.ticketWidthMm },
      data.settings.printerName,
      data.databaseId ?? '',
    ).catch(() => {});
}
export function PaperPreview({ document }: { document: BusinessDocument }) {
  return (
    <div className={`paper-preview ${document.kind === 'sale' ? 'paper-roll' : 'paper-a4'}`}>
      {document.pages.map((page, index) => (
        <figure key={index}>
          <svg
            role="img"
            aria-label={`${document.title}, página ${index + 1} de ${document.pages.length}`}
            viewBox={`0 0 ${page.width} ${page.height}`}
            style={{ width: `${(page.width / 72) * 25.4}mm` }}
            xmlns="http://www.w3.org/2000/svg"
          >
            <rect width={page.width} height={page.height} fill="#fff" />
            {page.operations.map((op, i) => {
              if (op.type === 'rect')
                return (
                  <rect
                    key={i}
                    x={op.x}
                    y={op.y}
                    width={op.width}
                    height={op.height}
                    fill={`rgb(${op.color.join(',')})`}
                  />
                );
              if (op.type === 'logo')
                return document.logoData ? (
                  <image
                    key={i}
                    x={op.x}
                    y={op.y}
                    width={op.width}
                    height={op.height}
                    href={`data:image/${document.logoData.startsWith('/9j/') ? 'jpeg' : 'png'};base64,${document.logoData}`}
                    preserveAspectRatio="xMidYMid meet"
                  />
                ) : null;
              return (
                <text
                  key={i}
                  x={op.x}
                  y={op.y}
                  fontSize={op.size}
                  fontWeight={op.bold ? 700 : 400}
                  textLength={op.width || undefined}
                  lengthAdjust="spacingAndGlyphs"
                  fill={`rgb(${op.color.join(',')})`}
                >
                  {op.text}
                </text>
              );
            })}
          </svg>
          {document.pages.length > 1 && (
            <figcaption>
              Página {index + 1} de {document.pages.length}
            </figcaption>
          )}
        </figure>
      ))}
    </div>
  );
}
export function DocumentDialog({
  request,
  data,
  onClose,
}: {
  request: DocumentRequest;
  data: Bootstrap;
  onClose: () => void;
}) {
  const [width, setWidth] = useState(request.widthMm ?? data.settings?.ticketWidthMm ?? 80);
  const [printer, setPrinter] = useState(data.settings?.printerName ?? '');
  const [printing, setPrinting] = useState(false);
  const [printError, setPrintError] = useState<UiFailure | null>(null);
  const [printJob, setPrintJob] = useState<PrintJob | null>(null);
  const active = useRef(true);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  const selection = { ...request, widthMm: width };
  const document = useResource(() => api('document', selection), [request.kind, request.id, width]);
  const printers = useResource(() => api('printers'), []);
  const exportOp = useOperation();
  const selected = printers.data?.find((p) => (printer ? p.name === printer : p.isDefault));
  const send = async () => {
    if (printing) return;
    setPrinting(true);
    setPrintError(null);
    setPrintJob(null);
    try {
      const result = await queueDocument(selection, printer, data.databaseId ?? '');
      if (active.current) setPrintJob(result);
    } catch (error) {
      if (active.current) setPrintError(normalizeError(error));
    } finally {
      if (active.current) setPrinting(false);
    }
  };
  return (
    <Modal
      open
      onClose={() => {
        if (!exportOp.busy) onClose();
      }}
      title={document.data?.title ?? 'Preparando documento'}
      description="Vista previa del documento. Puedes volver a imprimir sin registrar otra operación."
      className="document-dialog"
    >
      <div className="document-controls">
        {request.kind === 'sale' && (
          <Field label="Ancho del ticket">
            <select
              value={width}
              onChange={(e) => setWidth(Number(e.target.value))}
              disabled={exportOp.busy}
            >
              <option value={58}>58 mm</option>
              <option value={80}>80 mm</option>
            </select>
          </Field>
        )}
        <Field label="Impresora de Windows">
          <select
            value={printer}
            onChange={(e) => setPrinter(e.target.value)}
            disabled={exportOp.busy}
          >
            <option value="">Predeterminada de Windows</option>
            {printer && !printers.data?.some((p) => p.name === printer) && (
              <option value={printer}>{printer} · No disponible</option>
            )}
            {printers.data?.map((p) => (
              <option key={p.name} value={p.name}>
                {p.name}
                {p.isDefault ? ' · Predeterminada' : ''}
              </option>
            ))}
          </select>
        </Field>
        <Button
          size="icon"
          variant="outline"
          aria-label="Actualizar impresoras"
          disabled={printers.busy}
          onClick={printers.reload}
        >
          <RefreshCw size={17} />
        </Button>
      </div>
      <Failure error={printers.error} retry={printers.reload} />
      {selected?.virtualPrinter && (
        <p className="muted">
          Para esta impresora virtual, utiliza Guardar PDF. El archivo conservará el tamaño y el
          logotipo que ves aquí.
        </p>
      )}
      {!printers.busy && !selected && !printers.error && (
        <p className="muted">Selecciona una impresora disponible o guarda el documento como PDF.</p>
      )}
      {document.busy ? (
        <Busy>Preparando el papel…</Busy>
      ) : document.error ? (
        <Failure error={document.error} retry={document.reload} />
      ) : (
        document.data && <PaperPreview document={document.data} />
      )}
      {document.data?.warnings.map((warning) => (
        <p className="muted" key={warning}>
          {warning}
        </p>
      ))}
      <Failure error={exportOp.error} />
      <Failure error={printError} />
      {printJob && (
        <p className="document-status">
          <Check size={17} /> Enviado a {printJob.printer}. Trabajo {printJob.jobId}.
        </p>
      )}
      <div className="dialog-actions document-actions">
        <Button
          variant="outline"
          disabled={document.busy || !!document.error || exportOp.busy}
          onClick={() =>
            void exportOp.run(
              () => api('exportDocument', selection),
              (result) => {
                if (result)
                  toast.success('PDF guardado', { description: result.path, duration: 9000 });
              },
            )
          }
        >
          <FileDown size={17} />
          {exportOp.busy ? 'Guardando…' : 'Guardar PDF'}
        </Button>
        <Button
          variant="outline"
          disabled={
            printing ||
            exportOp.busy ||
            document.busy ||
            !!document.error ||
            !selected ||
            selected.virtualPrinter
          }
          onClick={() => void send()}
        >
          {printing ? <LoaderCircle className="spin" size={17} /> : <Printer size={17} />}{' '}
          {printing ? 'Enviando…' : 'Imprimir'}
        </Button>
        <Button disabled={exportOp.busy} onClick={onClose}>
          Continuar
          <ArrowRight size={17} />
        </Button>
      </div>
    </Modal>
  );
}
