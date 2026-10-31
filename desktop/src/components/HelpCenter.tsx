import { useCallback, useEffect, useRef, useState } from 'react';
import { driver, type Driver, type DriveStep } from 'driver.js';
import 'driver.js/dist/driver.css';
import { Play, RotateCcw, Sparkles, ArrowRight } from 'lucide-react';
import { toast } from 'sonner';
import { api, type Bootstrap, type Preferences } from '@/lib/api';
import { useVisual } from '@/lib/appearance';
import { Button, Modal, reportError } from './common';

function stepsFor(role: number): DriveStep[] {
  const steps =
    role === 0
      ? [
          [
            'metrics',
            'Tu negocio, de un vistazo',
            'Consulta ventas, efectivo y cuentas por cobrar. Cada cifra proviene de las operaciones guardadas.',
          ],
          [
            'charts',
            'Descubre cómo va tu negocio',
            'Cambia el periodo, recorre las gráficas y abre el detalle para entender tus resultados.',
          ],
          [
            'nav-inventory',
            'Inventario bajo control',
            'Busca por código, nombre, marca o medida. Revisa faltantes y consulta cada movimiento.',
          ],
          [
            'nav-cash',
            'Una caja clara',
            'Abre la jornada, registra ingresos o retiros y compara el efectivo contado al cerrar.',
          ],
          [
            'nav-settings',
            'Tu negocio y tus datos',
            'Personaliza los tickets, administra usuarios y crea respaldos desde Administración de datos.',
          ],
          [
            'help',
            'Siempre hay una guía a la mano',
            'Regresa aquí cuando lo necesites. También puedes abrir la ayuda con F1.',
          ],
        ]
      : [
          [
            'cash-status',
            'Antes de empezar',
            'Comprueba que la caja esté abierta. El fondo inicial es el dinero con el que inicia la jornada.',
          ],
          [
            'product-search',
            'Encuentra y agrega',
            'Escanea para agregar directamente. F2 abre la búsqueda por nombre, marca o medida; Enter elige y Escape vuelve a la venta. F3 enfoca la captura de códigos.',
          ],
          [
            'cart',
            'Un ticket fácil de revisar',
            'Ajusta cantidades y elige al cliente. Los importes se calculan con los precios del negocio.',
          ],
          [
            'checkout',
            'Cobra con confianza',
            'F8 abre el cobro. Revisa la forma de pago y el cambio antes de confirmar; el ticket se guarda primero.',
          ],
          [
            'cart-tools',
            'Continúa sin perder el ritmo',
            'Guarda una venta para después o crea una cotización. Reimprime desde el historial sin cobrar dos veces.',
          ],
          [
            'help',
            'Ya tienes lo esencial',
            'Abre F1 para consultar los atajos o repetir este recorrido cuando quieras.',
          ],
        ];
  return steps.map(([element, title, description]) => ({
    element: `[data-tour="${element}"]`,
    popover: {
      title,
      description,
      side: element.startsWith('nav-') || element === 'help' ? 'right' : 'bottom',
      align: 'center',
    },
    waitForElement: 5000,
  }));
}
export function useTutorial(data: Bootstrap) {
  const visual = useVisual();
  const [preferences, setPreferences] = useState(data.preferences);
  const instance = useRef<Driver | null>(null);
  const active = useRef(true);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  const persist = useCallback((status: Preferences['tutorialStatus'], step: number) => {
    chain.current = chain.current
      .catch(() => undefined)
      .then(() => (active.current ? api('tutorial', { status, step }) : null))
      .then((value) => {
        if (active.current && value) setPreferences(value);
      })
      .catch((error) => {
        if (active.current) reportError(error);
      });
  }, []);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
      clearTimeout(timer.current);
      instance.current?.destroy();
    };
  }, []);
  const start = (resume: boolean) => {
    instance.current?.destroy();
    const steps = stepsFor(data.actor!.role);
    const initial = resume ? Math.min(preferences?.tutorialStep ?? 0, steps.length - 1) : 0;
    const tour = driver({
      steps,
      animate: visual.duration > 0,
      duration: visual.duration * 1000,
      smoothScroll: visual.duration > 0,
      disableActiveInteraction: true,
      allowClose: true,
      overlayClickBehavior: 'none',
      showProgress: true,
      progressText: '{{current}} de {{total}}',
      nextBtnText: 'Siguiente',
      prevBtnText: 'Atrás',
      doneBtnText: 'Listo',
      closeBtnLabel: 'Pausar recorrido',
      popoverClass: 'donovan-tour',
      stagePadding: 7,
      stageRadius: 12,
      overlayOpacity: 0.6,
      onHighlighted: (_element, _step, options) => persist('in_progress', options.index ?? 0),
      onDestroyed: (_element, _step, options) => {
        if (!active.current) return;
        if (options.reason === 'done') {
          persist('completed', steps.length - 1);
          toast.success('Ya conoces lo esencial. Puedes repetir el recorrido desde Ayuda.');
        } else if (options.reason !== 'api' && options.reason !== 'replaced') {
          persist('in_progress', options.state.activeIndex ?? initial);
          toast.info('Recorrido pausado. Puedes continuarlo desde Ayuda.');
        }
      },
    });
    instance.current = tour;
    timer.current = setTimeout(() => {
      if (active.current) tour.drive(initial);
    }, 180);
  };
  return { preferences, start, skip: () => persist('skipped', 0) };
}
export function HelpCenter({
  open,
  data,
  preferences,
  onClose,
  onStart,
}: {
  open: boolean;
  data: Bootstrap;
  preferences: Preferences | null;
  onClose: () => void;
  onStart: (resume: boolean) => void;
}) {
  const pending = preferences?.tutorialStatus === 'pending';
  const resume = preferences?.tutorialStatus === 'in_progress';
  return (
    <Modal
      open={open}
      onClose={onClose}
      title={pending ? 'Bienvenido a tu nuevo espacio.' : 'Un apoyo para tu jornada.'}
      description={
        data.actor?.role === 0
          ? 'Conoce los controles que te ayudan a dirigir tu negocio.'
          : 'Lo esencial para atender con rapidez y confianza.'
      }
      className="help-dialog"
    >
      <div className="help-hero">
        <Sparkles size={30} />
        <div>
          <h3>
            {data.actor?.role === 0 ? 'Tu negocio, bajo control.' : 'Listo para tu próxima venta.'}
          </h3>
          <p>
            Un recorrido de 6 pasos, sin modificar ventas ni inventario. Puedes pausarlo en
            cualquier momento.
          </p>
        </div>
      </div>
      <div className="help-start">
        <Button onClick={() => onStart(resume)}>
          <Play size={16} />
          {resume
            ? `Continuar en el paso ${(preferences?.tutorialStep ?? 0) + 1}`
            : preferences?.tutorialStatus === 'completed'
              ? 'Repetir recorrido'
              : 'Conocer el sistema'}
          <ArrowRight size={16} />
        </Button>
        {resume && (
          <Button variant="outline" onClick={() => onStart(false)}>
            <RotateCcw size={16} /> Desde el inicio
          </Button>
        )}
        {pending && (
          <Button variant="ghost" onClick={onClose}>
            Ahora no
          </Button>
        )}
      </div>
      <h3>Atajos para avanzar más rápido</h3>
      <div className="shortcut-grid">
        {[
          ['F2', 'Buscar y agregar un producto'],
          ['F3', 'Volver a la captura de códigos'],
          ['F8', 'Abrir el cobro de la venta'],
          ['F1', 'Abrir esta ayuda'],
        ].map(([key, text]) => (
          <div key={key}>
            <kbd>{key}</kbd>
            <span>{text}</span>
          </div>
        ))}
      </div>
      <p className="notice">
        Una venta confirmada ya está guardada. Si la impresión falla, abre Ventas y reimprime el
        mismo folio. Para llevar el negocio a otro equipo, utiliza un respaldo completo.
      </p>
    </Modal>
  );
}
