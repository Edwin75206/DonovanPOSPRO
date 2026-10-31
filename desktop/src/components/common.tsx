import { Component, useRef, useState, type ReactNode, type RefObject } from 'react';
import {
  AlertCircle,
  ArrowRight,
  Check,
  ChevronLeft,
  LoaderCircle,
  PackageSearch,
  X,
} from 'lucide-react';
import { toast } from 'sonner';
import { Button } from './ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from './ui/dialog';
import { api, normalizeError, type UiFailure } from '@/lib/api';
export { Button };
let activeOperations = 0;
export const hasActiveOperation = () => activeOperations > 0;
export function reportError(error: unknown) {
  const e = normalizeError(error);
  toast.error(e.message, { description: `Referencia ${e.incidentId}`, duration: 7000 });
  return e;
}
export function useOperation() {
  const inFlight = useRef(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<UiFailure | null>(null);
  async function run<T>(action: () => Promise<T>, success?: (result: T) => void) {
    if (inFlight.current) return;
    inFlight.current = true;
    activeOperations++;
    let counted = true;
    setBusy(true);
    setError(null);
    try {
      const result = await action();
      activeOperations--;
      counted = false;
      success?.(result);
      return result;
    } catch (e) {
      setError(normalizeError(e));
      return undefined;
    } finally {
      inFlight.current = false;
      if (counted) activeOperations--;
      setBusy(false);
    }
  }
  return { busy, error, run, setError };
}
export function Failure({ error, retry }: { error: UiFailure | null; retry?: () => void }) {
  return error ? (
    <div className="failure" role="alert">
      <AlertCircle size={19} />
      <div>
        <strong>{error.message}</strong>
        <small>Referencia {error.incidentId}</small>
        {retry && (
          <Button variant="outline" onClick={retry}>
            Intentar nuevamente
          </Button>
        )}
      </div>
    </div>
  ) : null;
}
export function Busy({ children = 'Preparando tu espacio…' }: { children?: ReactNode }) {
  return (
    <div className="loading">
      <LoaderCircle className="spin" size={24} />
      <span>{children}</span>
    </div>
  );
}
export function Empty({
  title,
  children,
  action,
}: {
  title: string;
  children?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="empty">
      <div className="empty-icon">
        <PackageSearch size={30} />
      </div>
      <h3>{title}</h3>
      <p>{children}</p>
      {action}
    </div>
  );
}
export function Modal({
  open,
  onClose,
  title,
  description,
  children,
  className = '',
  initialFocusRef,
  returnFocusRef,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: ReactNode;
  className?: string;
  initialFocusRef?: RefObject<HTMLElement | null>;
  returnFocusRef?: RefObject<HTMLElement | null>;
}) {
  return (
    <Dialog
      open={open}
      onOpenChange={(v) => {
        if (!v) onClose();
      }}
    >
      <DialogContent
        className={`pos-dialog ${className}`}
        onOpenAutoFocus={(event) => {
          if (initialFocusRef) {
            event.preventDefault();
            initialFocusRef.current?.focus();
          }
        }}
        onCloseAutoFocus={(event) => {
          if (returnFocusRef) {
            event.preventDefault();
            returnFocusRef.current?.focus();
          }
        }}
      >
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {description && <DialogDescription>{description}</DialogDescription>}
        </DialogHeader>
        {children}
      </DialogContent>
    </Dialog>
  );
}
export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
      {hint && <small>{hint}</small>}
    </label>
  );
}
export function PageTitle({
  eyebrow,
  title,
  description,
  actions,
}: {
  eyebrow?: string;
  title: string;
  description?: string;
  actions?: ReactNode;
}) {
  return (
    <div className="page-title">
      <div>
        {eyebrow && <span className="eyebrow">{eyebrow}</span>}
        <h1>{title}</h1>
        {description && <p>{description}</p>}
      </div>
      <div className="page-actions">{actions}</div>
    </div>
  );
}
export class ErrorBoundary extends Component<
  { children: ReactNode },
  { failed: boolean; incidentId: string }
> {
  state = { failed: false, incidentId: '' };
  private failureSerial = 0;
  static getDerivedStateFromError() {
    return { failed: true, incidentId: '' };
  }
  componentDidCatch(error: Error) {
    /* Never persist component props or input values. */
    const serial = ++this.failureSerial;
    const kind =
      error instanceof TypeError
        ? 'TypeError'
        : error instanceof RangeError
          ? 'RangeError'
          : error instanceof ReferenceError
            ? 'ReferenceError'
            : error instanceof SyntaxError
              ? 'SyntaxError'
              : 'Error';
    void api('uiIncident', { kind })
      .then(({ incidentId }) => {
        if (serial === this.failureSerial && this.state.failed) this.setState({ incidentId });
      })
      .catch(() => {
        /* The recovery screen also works when the native bridge is unavailable. */
      });
  }
  render() {
    return this.state.failed ? (
      <div className="recovery">
        <AlertCircle size={32} />
        <h1>Esta pantalla necesita volver a cargar.</h1>
        <p>Las operaciones ya confirmadas siguen guardadas.</p>
        {this.state.incidentId && (
          <small>
            Referencia {this.state.incidentId} · Disponible en Administración de datos →
            Diagnóstico.
          </small>
        )}
        <Button
          onClick={() => {
            this.failureSerial++;
            this.setState({ failed: false, incidentId: '' });
            window.dispatchEvent(new CustomEvent('go-home'));
          }}
        >
          Volver al inicio <ArrowRight size={16} />
        </Button>
      </div>
    ) : (
      this.props.children
    );
  }
}
export { Check, ChevronLeft, X };
