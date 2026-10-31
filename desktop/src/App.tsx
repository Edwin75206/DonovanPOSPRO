import { useCallback, useEffect, useState, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { motion } from 'motion/react';
import {
  ShoppingBasket,
  Wallet,
  PanelLeftClose,
  PanelLeftOpen,
  Sun,
  Moon,
  LockKeyhole,
  LogOut,
  HelpCircle,
  ChevronDown,
  LayoutDashboard,
  Palette,
  ArrowLeft,
  SearchX,
  Package,
  Settings,
  FolderOpen,
} from 'lucide-react';
import { Toaster } from 'sonner';
import { api, type Bootstrap } from '@/lib/api';
import { VisualProvider, useVisual, type AppearanceState } from '@/lib/appearance';
import { Auth, Welcome, Logo } from '@/pages/Auth';
import { Register, freshCart, type Cart } from '@/pages/Register';
import { Cash } from '@/pages/Cash';
import { Dashboard, type Navigate } from '@/pages/Dashboard';
import { Appearance } from '@/pages/Appearance';
import { Inventory } from '@/pages/Inventory';
import { DataManager, RestoreDialog } from '@/pages/DataManager';
import { Busy, Button, Modal, ErrorBoundary, reportError, Failure } from '@/components/common';
import { TooltipProvider, Tooltip, TooltipTrigger, TooltipContent } from '@/components/ui/tooltip';

export default function App() {
  const [data, setData] = useState<Bootstrap | null>(null);
  const [initialError, setInitialError] = useState<ReturnType<typeof reportError> | null>(null);
  const [help, setHelp] = useState(false);
  const [carts, setCarts] = useState<Record<string, Cart>>({});
  const [restore, setRestore] = useState(false);
  const generation = useRef(0);
  const onSession = useCallback((next: Bootstrap) => {
    generation.current++;
    setInitialError(null);
    setData(next);
  }, []);
  const bootstrap = useCallback(() => {
    const current = ++generation.current;
    void api('bootstrap')
      .then((next) => {
        if (generation.current === current) {
          setInitialError(null);
          setData(next);
        }
      })
      .catch((e) => {
        if (generation.current === current) setInitialError(reportError(e));
      });
  }, []);
  const onDataChanged = useCallback(
    (next: Bootstrap) => {
      setCarts({});
      onSession(next);
    },
    [onSession],
  );
  const hasDraft = Object.values(carts).some((c) => c.rows.length > 0);
  useEffect(() => {
    bootstrap();
    window.addEventListener('session-ended', bootstrap);
    return () => window.removeEventListener('session-ended', bootstrap);
  }, [bootstrap]);
  useEffect(() => {
    let active = true;
    let off: (() => void) | undefined;
    void listen('storage-unavailable', bootstrap).then((stop) => {
      if (active) off = stop;
      else stop();
    });
    return () => {
      active = false;
      off?.();
    };
  }, [bootstrap]);
  useEffect(() => {
    if (!data?.sessionExpiresAt) return;
    const timer = window.setTimeout(
      bootstrap,
      Math.max(0, data.sessionExpiresAt * 1000 - Date.now() + 100),
    );
    return () => window.clearTimeout(timer);
  }, [data?.sessionExpiresAt, bootstrap]);
  const key = `${data?.databaseId ?? 'current'}:${data?.actor?.id ?? 0}`;
  const cart = carts[key] ?? freshCart();
  const setCart = useCallback(
    (value: Cart | ((v: Cart) => Cart)) =>
      setCarts((all) => ({
        ...all,
        [key]: typeof value === 'function' ? value(all[key] ?? freshCart()) : value,
      })),
    [key],
  );
  let content;
  if (initialError || data?.storageError)
    content = (
      <main className="recovery">
        <h1>Vamos a recuperar tu espacio.</h1>
        <p>
          No podemos acceder a la información del negocio. Conservamos tus archivos para poder
          revisarlos.
        </p>
        <Failure error={initialError ?? data?.storageError ?? null} />
        <Button
          onClick={() => {
            setInitialError(null);
            void api('retryStorage')
              .then(onSession)
              .catch((e) => setInitialError(reportError(e)));
          }}
        >
          Intentar nuevamente
        </Button>
        <Button variant="outline" onClick={() => setRestore(true)}>
          <FolderOpen size={17} />
          Restaurar una copia verificada
        </Button>
      </main>
    );
  else if (!data) content = <Busy />;
  else if (data.needsSetup)
    content = <Welcome data={data} onSession={onSession} onRestore={() => setRestore(true)} />;
  else if (!data.actor)
    content = <Auth data={data} onSession={onSession} onRestore={() => setRestore(true)} />;
  else
    content = (
      <Workspace
        key={key}
        data={data}
        onSession={onSession}
        onDataChanged={onDataChanged}
        hasDraft={hasDraft}
        onAppearance={(appearance) => setData({ ...data, appearance })}
        cart={cart}
        setCart={setCart}
        onHelp={() => setHelp(true)}
      />
    );
  return (
    <VisualProvider source={data?.appearance}>
      <TooltipProvider>
        {content}
        <RestoreDialog
          open={restore}
          onClose={() => setRestore(false)}
          data={data}
          onRestored={onDataChanged}
          hasDraft={hasDraft}
        />
        <Modal
          open={help}
          onClose={() => setHelp(false)}
          title="Ayuda y tutoriales"
          description="Un apoyo breve para tu jornada."
        >
          <div className="notice">
            <p>
              Para registrar una venta, abre caja, escanea el producto y presiona F8. El cobro se
              confirma antes de imprimir; puedes reimprimir desde el historial sin duplicar una
              venta.
            </p>
          </div>
        </Modal>
        <Toaster position="bottom-center" richColors closeButton duration={4500} />
      </TooltipProvider>
    </VisualProvider>
  );
}
function Workspace({
  data,
  onSession,
  onAppearance,
  onDataChanged,
  hasDraft,
  cart,
  setCart,
  onHelp,
}: {
  data: Bootstrap;
  onSession: (data: Bootstrap) => void;
  onAppearance: (a: AppearanceState) => void;
  onDataChanged: (data: Bootstrap) => void;
  hasDraft: boolean;
  cart: Cart;
  setCart: (value: Cart | ((v: Cart) => Cart)) => void;
  onHelp: () => void;
}) {
  const actor = data.actor!;
  const home = actor.role === 0 ? 'dashboard' : 'pos';
  const [page, setPage] = useState(home);
  const [filters, setFilters] = useState<Record<string, unknown>>({});
  const [userMenu, setUserMenu] = useState(false);
  const visual = useVisual();
  const compact = visual.appearance.config.sidebar === 'compact';
  const navigate: Navigate = useCallback((next, params = {}) => {
    setPage(next);
    setFilters(params);
  }, []);
  useEffect(() => {
    const goHome = () => navigate(home);
    const key = (e: KeyboardEvent) => {
      if (e.key === 'F2' && !document.querySelector('[role=dialog]')) {
        e.preventDefault();
        navigate('pos');
      }
      if (e.key === 'F1') {
        e.preventDefault();
        onHelp();
      }
    };
    window.addEventListener('go-home', goHome);
    window.addEventListener('keydown', key);
    return () => {
      window.removeEventListener('go-home', goHome);
      window.removeEventListener('keydown', key);
    };
  }, [home, navigate, onHelp]);
  const saveVisual = (config: AppearanceState['config']) =>
    void api('saveAppearance', { config, backgroundAction: 'keep', backgroundData: null })
      .then(onAppearance)
      .catch(reportError);
  const nav = [
    ...(actor.role === 0 ? [{ id: 'dashboard', name: 'Inicio', icon: LayoutDashboard }] : []),
    { id: 'pos', name: 'Punto de venta', icon: ShoppingBasket },
    { id: 'inventory', name: actor.role === 1 ? 'Productos' : 'Inventario', icon: Package },
    { id: 'cash', name: 'Caja', icon: Wallet },
    { id: 'appearance', name: 'Apariencia', icon: Palette },
    ...(actor.role === 0 ? [{ id: 'settings', name: 'Configuración', icon: Settings }] : []),
  ];
  const titles: Record<string, string> = {
    dashboard: 'Inicio',
    pos: 'Punto de venta',
    inventory: 'Productos e inventario',
    cash: 'Caja',
    appearance: 'Apariencia y personalización',
    settings: 'Administración de datos',
  };
  let pageContent;
  switch (page) {
    case 'dashboard':
      pageContent =
        actor.role === 0 ? (
          <Dashboard data={data} onNavigate={navigate} />
        ) : (
          <NotFound onHome={() => navigate(home)} />
        );
      break;
    case 'pos':
      pageContent = (
        <Register
          data={data}
          cart={cart}
          setCart={setCart}
          onCash={() => navigate('cash')}
          onSaved={() => {}}
        />
      );
      break;
    case 'inventory':
      pageContent = <Inventory actor={actor} filters={filters} />;
      break;
    case 'cash':
      pageContent = <Cash />;
      break;
    case 'appearance':
      pageContent = <Appearance data={data} onSaved={onAppearance} />;
      break;
    case 'settings':
      pageContent =
        actor.role === 0 ? (
          <DataManager data={data} onChanged={onDataChanged} hasDraft={hasDraft} />
        ) : (
          <NotFound onHome={() => navigate(home)} />
        );
      break;
    default:
      pageContent = <NotFound onHome={() => navigate(home)} />;
  }
  return (
    <div className={`app-shell ${compact ? 'compact' : ''}`}>
      <aside className="sidebar" data-tour="navigation">
        <div className="business-identity">
          <Logo data={data} />
          <div>
            <strong>{data.settings?.storeName}</strong>
            <small>{actor.role === 0 ? 'ADMINISTRADOR' : 'MOSTRADOR'}</small>
          </div>
        </div>
        <nav className="sidebar-nav">
          <span className="nav-group-label">{actor.role === 0 ? 'MI NEGOCIO' : 'MOSTRADOR'}</span>
          {nav.map((item) => (
            <Tooltip key={item.id}>
              <TooltipTrigger asChild>
                <button
                  aria-label={item.name}
                  aria-current={page === item.id ? 'page' : undefined}
                  className={page === item.id ? 'active' : ''}
                  onClick={() => navigate(item.id)}
                >
                  {page === item.id && (
                    <motion.span
                      layoutId="nav-active"
                      className="nav-active"
                      transition={{ duration: visual.duration }}
                    />
                  )}
                  <item.icon size={17} />
                  <span>{item.name}</span>
                  {item.id === 'pos' && cart.rows.length > 0 && (
                    <small className="cart-count">{cart.rows.length}</small>
                  )}
                </button>
              </TooltipTrigger>
              {compact && <TooltipContent side="right">{item.name}</TooltipContent>}
            </Tooltip>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <button
            className="sidebar-help"
            title="Ayuda y tutoriales"
            onClick={onHelp}
            data-tour="help"
          >
            <HelpCircle size={17} />
            <span>Ayuda y tutoriales</span>
          </button>
          <div className="sidebar-product">
            <b>D</b>
            <span>DONOVAN POS PRO</span>
          </div>
        </div>
      </aside>
      <div className="main-workspace">
        <header className="app-header">
          <div className="header-location">
            <button
              className="icon-button"
              disabled={page === 'appearance'}
              aria-label={compact ? 'Expandir menú' : 'Contraer menú'}
              onClick={() =>
                saveVisual({
                  ...visual.appearance.config,
                  sidebar: compact ? 'expanded' : 'compact',
                })
              }
            >
              {compact ? <PanelLeftOpen size={18} /> : <PanelLeftClose size={18} />}
            </button>
            <span className="header-business">{data.settings?.storeName}</span>
            <span>/</span>
            <b>{titles[page] ?? 'Sección'}</b>
          </div>
          <div className="header-actions">
            <span className="local-tag">Todo en tu equipo</span>
            <span className="header-date">
              {new Intl.DateTimeFormat('es-MX', {
                weekday: 'long',
                day: 'numeric',
                month: 'short',
              }).format(new Date())}
            </span>
            <button
              className="icon-button"
              onClick={() =>
                saveVisual({ ...visual.appearance.config, mode: visual.dark ? 'light' : 'dark' })
              }
              aria-label="Cambiar tema"
              disabled={page === 'appearance'}
            >
              {visual.dark ? <Sun size={17} /> : <Moon size={17} />}
            </button>
            <button
              className="icon-button"
              aria-label="Bloquear pantalla"
              title="Bloquear pantalla"
              onClick={() => void api('lock').then(onSession).catch(reportError)}
            >
              <LockKeyhole size={17} />
            </button>
            <button
              className="user-button"
              onClick={() => setUserMenu(true)}
              aria-label={`Cuenta de ${actor.name}`}
            >
              <span className="avatar">{actor.name.slice(0, 2).toUpperCase()}</span>
              <ChevronDown size={14} />
            </button>
          </div>
        </header>
        {data.demo && (
          <div className="demo-strip">DEMOSTRACIÓN · Datos de ejemplo separados de tu negocio</div>
        )}
        <div className="page-container">
          <ErrorBoundary>{pageContent}</ErrorBoundary>
        </div>
        <footer className="app-statusbar">
          <span>
            MXN · Español (México)<b>Donovan Systems</b>
          </span>
          <span>
            F1 Ayuda · F2 Venta · F3 Buscar · F8 Cobrar<span>v{data.version}</span>
          </span>
        </footer>
      </div>
      <Modal
        open={userMenu}
        onClose={() => setUserMenu(false)}
        title={actor.name}
        description={
          actor.role === 0 ? 'Administrador' : actor.role === 2 ? 'Encargado' : 'Vendedor'
        }
      >
        <Button
          variant="outline"
          onClick={() => {
            setUserMenu(false);
            navigate('appearance');
          }}
        >
          <Palette size={16} />
          Personalizar mi espacio
        </Button>
        <Button
          variant="outline"
          onClick={() => {
            setUserMenu(false);
            onHelp();
          }}
        >
          <HelpCircle size={16} />
          Ayuda y tutoriales
        </Button>
        <Button
          variant="outline"
          onClick={() => void api('lock').then(onSession).catch(reportError)}
        >
          <LockKeyhole size={16} />
          Bloquear pantalla
        </Button>
        <Button
          variant="outline"
          onClick={() => void api('logout').then(onSession).catch(reportError)}
        >
          <LogOut size={16} />
          Cerrar sesión / cambiar de usuario
        </Button>
      </Modal>
    </div>
  );
}
export function NotFound({ onHome }: { onHome: () => void }) {
  return (
    <main className="recovery">
      <SearchX size={44} strokeWidth={1.5} />
      <span className="eyebrow">SECCIÓN NO DISPONIBLE</span>
      <h1>No encontramos esta sección.</h1>
      <p>Vuelve al inicio para continuar con tu negocio.</p>
      <Button onClick={onHome}>
        <ArrowLeft size={17} />
        Volver al inicio
      </Button>
    </main>
  );
}
