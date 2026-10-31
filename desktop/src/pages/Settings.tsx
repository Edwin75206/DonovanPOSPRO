import { useEffect, useState } from 'react';
import {
  Building2,
  Database,
  Users,
  Info,
  ImagePlus,
  Save,
  Plus,
  Pencil,
  ShieldCheck,
  RefreshCw,
  Printer,
} from 'lucide-react';
import { toast } from 'sonner';
import { api, type Actor, type Bootstrap } from '@/lib/api';
import { useResource } from '@/lib/useResource';
import { parseScaled } from '@/lib/utils';
import {
  Button,
  Failure,
  Field,
  Modal,
  PageTitle,
  useOperation,
  reportError,
} from '@/components/common';
import { DataGrid } from '@/components/DataGrid';
import { DataManager } from './DataManager';
import { PrinterSettings } from './PrinterSettings';
import { Logo } from './Auth';
import { useLeaveGuard } from '@/lib/leaveGuard';

const sections = [
  { id: 'business', label: 'Datos del negocio', icon: Building2 },
  { id: 'printers', label: 'Impresoras y tickets', icon: Printer },
  { id: 'data', label: 'Administración de datos', icon: Database },
  { id: 'users', label: 'Usuarios y permisos', icon: Users },
  { id: 'about', label: 'Acerca de', icon: Info },
];
export function Settings({
  data,
  onChanged,
  onSession,
  hasDraft,
  filters,
}: {
  data: Bootstrap;
  onChanged: (next: Bootstrap) => void;
  onSession: (next: Bootstrap) => void;
  hasDraft: boolean;
  filters: Record<string, unknown>;
}) {
  const [section, setSection] = useState(
    typeof filters.tab === 'string' && sections.some((s) => s.id === filters.tab)
      ? filters.tab
      : 'business',
  );
  const [dirty, setDirty] = useState(false);
  const [nextSection, setNextSection] = useState<string | null>(null);
  const leaveGuard = useLeaveGuard(dirty);
  useEffect(() => {
    if (typeof filters.tab === 'string' && sections.some((s) => s.id === filters.tab))
      setSection(filters.tab);
  }, [filters]);
  const select = (id: string) => {
    if (id === section) return;
    if (dirty) setNextSection(id);
    else setSection(id);
  };
  return (
    <div className="settings-workspace">
      <nav className="settings-tabs" aria-label="Configuración del negocio">
        {sections.map((s) => (
          <button
            key={s.id}
            aria-current={section === s.id ? 'page' : undefined}
            className={section === s.id ? 'active' : ''}
            onClick={() => select(s.id)}
          >
            <s.icon size={17} />
            {s.label}
          </button>
        ))}
      </nav>
      {leaveGuard}
      <div className="settings-content">
        {section === 'business' ? (
          <BusinessSettings data={data} onSaved={onSession} onDirty={setDirty} />
        ) : section === 'printers' ? (
          <PrinterSettings data={data} onSaved={onSession} onDirty={setDirty} />
        ) : section === 'data' ? (
          <DataManager data={data} onChanged={onChanged} hasDraft={hasDraft} />
        ) : section === 'users' ? (
          <UserSettings data={data} onSession={onSession} />
        ) : (
          <About version={data.version} />
        )}
      </div>
      <Modal
        open={!!nextSection}
        onClose={() => setNextSection(null)}
        title="Hay cambios sin guardar"
        description="Puedes seguir editando o salir y conservar la configuración guardada."
      >
        <div className="dialog-actions">
          <Button variant="outline" onClick={() => setNextSection(null)}>
            Seguir editando
          </Button>
          <Button
            onClick={() => {
              setDirty(false);
              setSection(nextSection!);
              setNextSection(null);
            }}
          >
            Salir sin guardar
          </Button>
        </div>
      </Modal>
    </div>
  );
}
function BusinessSettings({
  data,
  onSaved,
  onDirty,
}: {
  data: Bootstrap;
  onSaved: (next: Bootstrap) => void;
  onDirty: (dirty: boolean) => void;
}) {
  const source = data.settings!;
  const [draft, setDraft] = useState(source);
  const [logoAction, setLogoAction] = useState<'keep' | 'replace'>('keep');
  const op = useOperation();
  const dirty = Object.keys(source).some(
    (key) => draft[key as keyof typeof draft] !== source[key as keyof typeof source],
  );
  useEffect(() => {
    setDraft(source);
    setLogoAction('keep');
  }, [source]);
  useEffect(() => {
    onDirty(dirty);
    return () => onDirty(false);
  }, [dirty, onDirty]);
  return (
    <main className="page-scroll page-container">
      <PageTitle
        eyebrow="LA IDENTIDAD DE TU NEGOCIO"
        title="Cada detalle habla de ti."
        description="Estos datos aparecerán en tus tickets, cotizaciones y reportes."
      />
      <form
        className="business-settings-layout"
        onSubmit={(e) => {
          e.preventDefault();
          void op.run(
            () => api('saveBusiness', { ...draft, logoAction }),
            (next) => {
              toast.success('Configuración del negocio guardada.');
              onSaved(next);
            },
          );
        }}
      >
        <section className="surface settings-card">
          <fieldset disabled={op.busy}>
            <h2>Datos del negocio</h2>
            <div className="settings-logo">
              <Logo data={{ ...data, settings: draft }} />
              <div>
                <strong>Logotipo comercial</strong>
                <p>PNG con transparencia o JPEG, hasta 5 MB.</p>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() =>
                    void op.run(
                      () => api('chooseBusinessLogo'),
                      (image) => {
                        if (image) {
                          setDraft({ ...draft, logoData: image.data });
                          setLogoAction('replace');
                        }
                      },
                    )
                  }
                >
                  <ImagePlus size={16} /> Elegir logotipo
                </Button>
              </div>
            </div>
            <Field label="Nombre del negocio">
              <input
                required
                maxLength={150}
                value={draft.storeName}
                onChange={(e) => setDraft({ ...draft, storeName: e.target.value })}
              />
            </Field>
            <Field
              label="Subtítulo comercial"
              hint="Por ejemplo: Materiales, maquinaria y acarreos."
            >
              <input
                maxLength={300}
                value={draft.subtitle}
                onChange={(e) => setDraft({ ...draft, subtitle: e.target.value })}
              />
            </Field>
            <Field label="Dirección">
              <textarea
                maxLength={500}
                value={draft.address}
                onChange={(e) => setDraft({ ...draft, address: e.target.value })}
              />
            </Field>
            <div className="form-grid">
              <Field label="Teléfono">
                <input
                  type="tel"
                  maxLength={60}
                  value={draft.phone}
                  onChange={(e) => setDraft({ ...draft, phone: e.target.value })}
                />
              </Field>
              <Field label="Correo">
                <input
                  type="email"
                  maxLength={200}
                  value={draft.email}
                  onChange={(e) => setDraft({ ...draft, email: e.target.value })}
                />
              </Field>
            </div>
            <Field label="RFC">
              <input
                maxLength={30}
                value={draft.taxId}
                onChange={(e) => setDraft({ ...draft, taxId: e.target.value.toUpperCase() })}
              />
            </Field>
          </fieldset>
        </section>
        <section className="surface settings-card">
          <fieldset disabled={op.busy}>
            <h2>Tickets y operación</h2>
            <div className="form-grid">
              <Field label="Ancho del ticket">
                <select
                  value={draft.ticketWidthMm}
                  onChange={(e) => setDraft({ ...draft, ticketWidthMm: Number(e.target.value) })}
                >
                  <option value={80}>80 mm · Mayor espacio</option>
                  <option value={58}>58 mm · Compacto</option>
                </select>
              </Field>
              <Field label="Prefijo de los tickets" hint="La secuencia de folios se conserva.">
                <input
                  required
                  maxLength={8}
                  pattern="[A-Za-z0-9]{1,8}"
                  value={draft.folioPrefix}
                  onChange={(e) =>
                    setDraft({ ...draft, folioPrefix: e.target.value.toUpperCase() })
                  }
                />
              </Field>
            </div>
            <Field
              label="Mensaje al final del ticket"
              hint="Agradecimiento, condiciones de cambio o información para tu cliente."
            >
              <textarea
                maxLength={1500}
                rows={4}
                value={draft.ticketFooter}
                onChange={(e) => setDraft({ ...draft, ticketFooter: e.target.value })}
              />
            </Field>
            <Field
              label="IVA predeterminado para productos nuevos (%)"
              hint="Los precios incluyen IVA. Este valor no modifica productos existentes."
            >
              <input
                type="number"
                required
                min="0"
                max="100"
                step=".01"
                value={draft.defaultTaxBasisPoints / 100}
                onChange={(e) =>
                  setDraft({ ...draft, defaultTaxBasisPoints: parseScaled(e.target.value) })
                }
              />
            </Field>
            <div className="notice">
              <ShieldCheck size={19} />
              <p>
                La apariencia personal no cambia el papel impreso. Los documentos siempre usan el
                logotipo y los datos del negocio.
              </p>
            </div>
            <p className="muted">Los tickets son comprobantes de venta; no sustituyen un CFDI.</p>
            <p className="muted">
              Configura el papel, márgenes, copias y vista previa en Impresoras y tickets.
            </p>
          </fieldset>
        </section>
        <div className="settings-savebar">
          <Failure error={op.error} />
          <span>
            {dirty ? 'Tienes cambios listos para guardar.' : 'La configuración está guardada.'}
          </span>
          <div>
            <Button
              type="button"
              variant="outline"
              disabled={op.busy || !dirty}
              onClick={() => {
                setDraft(source);
                setLogoAction('keep');
              }}
            >
              Cancelar cambios
            </Button>
            <Button type="submit" disabled={op.busy || !dirty}>
              <Save size={17} />
              {op.busy ? 'Guardando…' : 'Guardar configuración'}
            </Button>
          </div>
        </div>
      </form>
    </main>
  );
}
const roles = ['Administrador', 'Vendedor', 'Encargado'];
const roleDescriptions = [
  'Control del negocio, reportes, usuarios, respaldos, devoluciones y configuración.',
  'Ventas, caja, productos de consulta, cotizaciones y sus propios tickets. Sin acceso a costos, crédito ni administración.',
  'Operación de mostrador, compras, inventario, costos, descuentos y crédito. Sin administrar usuarios, respaldos o devoluciones.',
];
type User = Actor & { active: boolean };
function UserSettings({
  data,
  onSession,
}: {
  data: Bootstrap;
  onSession: (next: Bootstrap) => void;
}) {
  const list = useResource(() => api('users'), []);
  const [editing, setEditing] = useState<User | null>(null);
  return (
    <main className="page-scroll page-container records-page">
      <PageTitle
        eyebrow="CADA PERSONA, SU ESPACIO"
        title="Un equipo con acceso claro."
        description="Cada usuario conserva su apariencia y tutorial. Los permisos se comprueban en cada operación."
        actions={
          <Button
            onClick={() => setEditing({ id: 0, username: '', name: '', role: 1, active: true })}
          >
            <Plus size={17} /> Nuevo usuario
          </Button>
        }
      />
      <Failure error={list.error} retry={list.reload} />
      <section className="surface">
        <DataGrid
          rows={list.data ?? []}
          caption="Usuarios del negocio"
          columns={[
            {
              id: 'name',
              label: 'Persona',
              value: (u) => u.name,
              render: (u) => (
                <div className="record-name">
                  <span className="record-avatar">
                    <Users size={18} />
                  </span>
                  <div>
                    <strong>{u.name}</strong>
                    <small>
                      {u.username}
                      {u.id === data.actor?.id ? ' · Tu cuenta' : ''}
                    </small>
                  </div>
                </div>
              ),
            },
            { id: 'role', label: 'Rol', value: (u) => roles[u.role] },
            {
              id: 'active',
              label: 'Estado',
              value: (u) => (u.active ? 'Activo' : 'Desactivado'),
              render: (u) => (
                <span className={`record-state ${u.active ? 'valid' : 'expired'}`}>
                  {u.active ? 'Activo' : 'Desactivado'}
                </span>
              ),
            },
            {
              id: 'actions',
              label: 'Administrar',
              value: (u) => u.id,
              sortable: false,
              render: (u) => (
                <Button size="sm" variant="outline" onClick={() => setEditing(u)}>
                  <Pencil size={15} /> Editar acceso
                </Button>
              ),
            },
          ]}
        />
      </section>
      <div className="role-cards">
        {roles.map((name, i) => (
          <article className="surface" key={name}>
            <ShieldCheck size={21} />
            <h3>{name}</h3>
            <p>{roleDescriptions[i]}</p>
          </article>
        ))}
      </div>
      <p className="muted">
        Las sesiones duran un máximo de 24 horas. Cambiar una contraseña, rol o estado cierra las
        sesiones de esa cuenta. No se puede desactivar al último administrador.
      </p>
      {editing && (
        <UserEditor
          key={editing.id}
          user={editing}
          currentId={data.actor!.id}
          onClose={() => setEditing(null)}
          onSaved={(reauthenticate) => {
            setEditing(null);
            list.reload();
            if (reauthenticate) void api('bootstrap').then(onSession).catch(reportError);
          }}
        />
      )}
    </main>
  );
}
function UserEditor({
  user,
  currentId,
  onClose,
  onSaved,
}: {
  user: User;
  currentId: number;
  onClose: () => void;
  onSaved: (reauthenticate: boolean) => void;
}) {
  const [draft, setDraft] = useState(user);
  const [password, setPassword] = useState('');
  const [repeat, setRepeat] = useState('');
  const [adminPassword, setAdminPassword] = useState('');
  const op = useOperation();
  return (
    <Modal
      open
      onClose={() => {
        if (!op.busy) onClose();
      }}
      title={user.id ? `Acceso de ${user.name}` : 'Nuevo integrante'}
      description="Define su acceso y confirma el cambio con tu contraseña de administrador."
    >
      <form
        className="record-form"
        onSubmit={(e) => {
          e.preventDefault();
          if (password !== repeat) return;
          void op.run(
            () => api('saveUser', { ...draft, password, adminPassword }),
            (result) => {
              setPassword('');
              setRepeat('');
              setAdminPassword('');
              toast.success('Usuario guardado.');
              onSaved(result.reauthenticate);
            },
          );
        }}
      >
        <fieldset disabled={op.busy}>
          <div className="form-grid">
            <Field label="Nombre completo">
              <input
                autoFocus
                required
                maxLength={100}
                value={draft.name}
                onChange={(e) => setDraft({ ...draft, name: e.target.value })}
              />
            </Field>
            <Field label="Usuario para iniciar sesión">
              <input
                required
                maxLength={50}
                autoComplete="off"
                value={draft.username}
                onChange={(e) => setDraft({ ...draft, username: e.target.value.toLowerCase() })}
              />
            </Field>
          </div>
          <Field label="Rol">
            <select
              disabled={user.id === currentId}
              value={draft.role}
              onChange={(e) => setDraft({ ...draft, role: Number(e.target.value) })}
            >
              {roles.map((r, i) => (
                <option value={i} key={r}>
                  {r}
                </option>
              ))}
            </select>
          </Field>
          <p className="notice">{roleDescriptions[draft.role]}</p>
          <div className="form-grid">
            <Field
              label={user.id ? 'Nueva contraseña (opcional)' : 'Contraseña'}
              hint="Mínimo 10 caracteres; combina letras y números."
            >
              <input
                type="password"
                autoComplete="new-password"
                required={!user.id}
                minLength={10}
                maxLength={128}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </Field>
            <Field label="Repite la contraseña">
              <input
                type="password"
                autoComplete="new-password"
                required={!!password || !user.id}
                maxLength={128}
                value={repeat}
                onChange={(e) => setRepeat(e.target.value)}
              />
            </Field>
          </div>
          {repeat && repeat !== password && (
            <p className="amount-pending">Las contraseñas no coinciden.</p>
          )}
          {user.id !== currentId && (
            <label className="check-label">
              <input
                type="checkbox"
                checked={draft.active}
                onChange={(e) => setDraft({ ...draft, active: e.target.checked })}
              />{' '}
              Cuenta activa
            </label>
          )}
          <Field label="Tu contraseña de administrador">
            <input
              type="password"
              required
              autoComplete="current-password"
              maxLength={512}
              value={adminPassword}
              onChange={(e) => setAdminPassword(e.target.value)}
            />
          </Field>
          {user.id === currentId && (
            <p className="muted">
              Al guardar tu propia cuenta tendrás que iniciar sesión nuevamente.
            </p>
          )}
        </fieldset>
        <Failure error={op.error} />
        <div className="dialog-actions">
          <Button type="button" variant="outline" disabled={op.busy} onClick={onClose}>
            Cancelar
          </Button>
          <Button type="submit" disabled={op.busy || password !== repeat}>
            {op.busy ? 'Guardando acceso…' : 'Guardar acceso'}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
function About({ version }: { version: string }) {
  return (
    <main className="page-scroll page-container">
      <PageTitle
        eyebrow="DONOVAN SYSTEMS"
        title="Tecnología al servicio de tu negocio."
        description="Donovan POS Pro · Aplicación de escritorio para Windows"
      />
      <section className="surface about-card">
        <div className="about-mark">D</div>
        <h2>Donovan POS Pro</h2>
        <span className="record-state">Versión {version}</span>
        <p>
          Desarrollado por <strong>Edwin Donovan Castañeda Calderón</strong>
        </p>
        <p>
          Tu información se almacena en esta computadora. Las ventas, el inventario y los respaldos
          funcionan sin conexión a internet.
        </p>
        <div className="notice">
          <Database size={22} />
          <p>
            Para cambiar de equipo, crea un respaldo completo y restáuralo en la nueva instalación
            desde Administración de datos.
          </p>
        </div>
        <small>© 2026 Donovan Systems. Todos los derechos reservados.</small>
      </section>
    </main>
  );
}
