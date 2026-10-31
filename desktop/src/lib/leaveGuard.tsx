import { useEffect, useState } from 'react';
import { Button, Modal, hasActiveOperation } from '@/components/common';
import { toast } from 'sonner';

export function requestWorkspaceLeave(proceed: () => void) {
  if (hasActiveOperation()) {
    toast.info('Espera a que termine la operación antes de cambiar de sección.');
    return;
  }
  const event = new CustomEvent('workspace-before-leave', {
    cancelable: true,
    detail: { proceed },
  });
  if (window.dispatchEvent(event)) proceed();
}
export function useLeaveGuard(dirty: boolean) {
  const [continuation, setContinuation] = useState<(() => void) | null>(null);
  useEffect(() => {
    if (!dirty) return;
    const beforeLeave = (event: Event) => {
      if (event.defaultPrevented) return;
      event.preventDefault();
      setContinuation(() => (event as CustomEvent<{ proceed: () => void }>).detail.proceed);
    };
    window.addEventListener('workspace-before-leave', beforeLeave);
    return () => window.removeEventListener('workspace-before-leave', beforeLeave);
  }, [dirty]);
  return (
    <Modal
      open={!!continuation}
      onClose={() => setContinuation(null)}
      title="Hay cambios sin guardar"
      description="Puedes seguir editando o salir y conservar lo que ya estaba guardado."
    >
      <div className="dialog-actions">
        <Button variant="outline" onClick={() => setContinuation(null)}>
          Seguir editando
        </Button>
        <Button
          onClick={() => {
            const proceed = continuation;
            setContinuation(null);
            proceed?.();
          }}
        >
          Salir sin guardar
        </Button>
      </div>
    </Modal>
  );
}
