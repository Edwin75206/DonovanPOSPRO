import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import {
  Database,
  ShieldCheck,
  HardDriveDownload,
  FolderOpen,
  RefreshCw,
  MonitorUp,
  Check,
  FilePlus2,
  ArrowRight,
  Clock3,
} from 'lucide-react';
import {
  api,
  type Bootstrap,
  type DataOverview,
  type RecoveryPreview,
  type Diagnostic,
} from '@/lib/api';
import {
  Button,
  Busy,
  Empty,
  Failure,
  Field,
  Modal,
  PageTitle,
  reportError,
  useOperation,
} from '@/components/common';
import { DataGrid } from '@/components/DataGrid';
import { ImportProducts } from '@/pages/ImportProducts';

const bytes = (n: number) =>
  n >= 1024 ** 3 ? `${(n / 1024 ** 3).toFixed(1)} GB` : `${(n / 1024 ** 2).toFixed(1)} MB`;
const date = (s: string | null) => {
  if (!s) return 'Sin comprobar';
  const value = new Date(s.replace(' ', 'T'));
  return Number.isNaN(value.getTime())
    ? 'Fecha no disponible'
    : new Intl.DateTimeFormat('es-MX', { dateStyle: 'medium', timeStyle: 'short' }).format(value);
};
export function RestoreDialog({
  open,
  onClose,
  data,
  onRestored,
  hasDraft = false,
}: {
  open: boolean;
  onClose: () => void;
  data: Bootstrap | null;
  onRestored: (data: Bootstrap) => void;
  hasDraft?: boolean;
}) {
  const operation = useOperation();
  const [preview, setPreview] = useState<RecoveryPreview | null>(null);
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [agreed, setAgreed] = useState(false);
  const needsLogin = !!data && !data.needsSetup && !data.storageError && !data.actor;
  const close = () => {
    if (operation.busy) return;
    if (preview) void api('cancelRecovery').catch(reportError);
    setPreview(null);
    setPassword('');
    setAgreed(false);
    operation.setError(null);
    onClose();
  };
  const choose = () =>
    void operation.run(
      () => api('chooseRecoverySource'),
      (p) => {
        if (p) {
          setPreview(p);
          setUsername(p.adminAccounts[0]?.username ?? '');
          setPassword('');
          setAgreed(false);
        }
      },
    );
  return (
    <Modal
      open={open}
      onClose={close}
      title="Recuperar tu negocio"
      description="Selecciona una copia, revisa su contenido y confirma la restauración."
      className="data-dialog"
    >
      <Failure error={operation.error} />
      {needsLogin ? (
        <div className="notice">
          Inicia sesión con una cuenta administradora. Después abre Configuración → Administración
          de datos para cambiar la base de este negocio.
        </div>
      ) : !preview ? (
        <div className="restore-start">
          <div className="large-feature-icon">
            <HardDriveDownload size={32} />
          </div>
          <h3>Tus datos viajan contigo.</h3>
          <p>
            Admite respaldos de Donovan POS Pro y bases SQLite compatibles. Revisaremos su
            estructura, integridad y contenido antes de activarlos.
          </p>
          <Button onClick={choose} disabled={operation.busy}>
            <FolderOpen size={17} />
            {operation.busy ? 'Validando la copia…' : 'Seleccionar respaldo o base'}
          </Button>
          <small>El archivo que selecciones se conserva sin modificaciones.</small>
        </div>
      ) : (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (!agreed) return;
            void operation.run(
              () => api('confirmRecovery', { planId: preview.planId, username, password }),
              (result) => {
                setPassword('');
                setPreview(null);
                onRestored(result);
                onClose();
                toast.success(
                  'La información se restauró y verificó. Inicia sesión para continuar.',
                );
              },
            );
          }}
        >
          <div className="restore-verified">
            <ShieldCheck size={26} />
            <div>
              <strong>Copia compatible e íntegra</strong>
              <span>{preview.fileName}</span>
            </div>
            <Button
              type="button"
              size="sm"
              variant="outline"
              disabled={operation.busy}
              onClick={choose}
            >
              Cambiar archivo
            </Button>
          </div>
          <h3>{preview.storeName}</h3>
          <div className="data-counts">
            {[
              [preview.counts.products, 'productos'],
              [preview.counts.sales, 'ventas'],
              [preview.counts.customers, 'clientes'],
              [preview.counts.users, 'usuarios'],
            ].map(([n, l]) => (
              <div key={l}>
                <strong>{n}</strong>
                <span>{l}</span>
              </div>
            ))}
          </div>
          <p className="muted">
            {bytes(preview.bytes)} ·{' '}
            {preview.createdAt ? date(preview.createdAt) : 'Base SQLite existente'} ·{' '}
            {preview.hasLogo ? 'Incluye logotipo' : 'Sin logotipo incrustado'}
          </p>
          {preview.legacy && (
            <div className="notice">
              Se aplicarán las migraciones de esta versión sobre una copia. Se conservarán tus
              cuentas, contraseñas y movimientos.
            </div>
          )}
          <div className="notice">
            {preview.authorization === 'current-admin'
              ? 'Se guardará un respaldo preventivo del negocio actual y se conservará su base anterior.'
              : 'Utiliza una cuenta administradora del negocio incluido en el respaldo.'}{' '}
            Las sesiones se cerrarán y la impresora deberá seleccionarse en este equipo.
          </div>
          {hasDraft && (
            <div className="notice warning">
              Hay carritos en preparación. Al cambiar de base se descartarán los que no hayas
              suspendido.
            </div>
          )}
          {preview.authorization === 'backup-admin' && (
            <Field label="Usuario administrador del respaldo">
              <input
                autoComplete="username"
                value={username}
                maxLength={50}
                required
                onChange={(e) => setUsername(e.target.value)}
                list="backup-admins"
              />
              <datalist id="backup-admins">
                {preview.adminAccounts.map((a) => (
                  <option key={a.username} value={a.username}>
                    {a.name}
                  </option>
                ))}
              </datalist>
            </Field>
          )}
          <Field
            label={
              preview.authorization === 'current-admin'
                ? 'Tu contraseña de administrador'
                : 'Contraseña de esa cuenta'
            }
          >
            <input
              type="password"
              autoComplete="current-password"
              maxLength={128}
              required
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
          </Field>
          <label className="check-row">
            <input type="checkbox" checked={agreed} onChange={(e) => setAgreed(e.target.checked)} />
            Quiero activar esta copia como el negocio de esta instalación.
          </label>
          <div className="dialog-actions">
            <Button type="button" variant="outline" disabled={operation.busy} onClick={close}>
              Cancelar
            </Button>
            <Button type="submit" disabled={operation.busy || !agreed}>
              <ShieldCheck size={17} />
              {operation.busy ? 'Respaldando y restaurando…' : 'Restaurar y comprobar'}
            </Button>
          </div>
        </form>
      )}
    </Modal>
  );
}
export function DataManager({
  data,
  onChanged,
  hasDraft,
}: {
  data: Bootstrap;
  onChanged: (data: Bootstrap) => void;
  hasDraft: boolean;
}) {
  const [overview, setOverview] = useState<DataOverview | null>(null);
  const load = useOperation();
  const operation = useOperation();
  const [restore, setRestore] = useState(false);
  const [options, setOptions] = useState(false);
  const [fresh, setFresh] = useState(false);
  const [transfer, setTransfer] = useState(false);
  const [diagnostics, setDiagnostics] = useState<Diagnostic[] | null>(null);
  const [lastFile, setLastFile] = useState('');
  const [importing, setImporting] = useState(false);
  const reload = useCallback(() => {
    void api('dataOverview').then(setOverview).catch(reportError);
  }, []);
  useEffect(() => {
    void load.run(() => api('dataOverview'), setOverview);
    let off: (() => void) | undefined;
    let active = true;
    void listen('backup-status-changed', () => {
      if (active) reload();
    }).then((unlisten) => {
      if (active) off = unlisten;
      else unlisten();
    });
    return () => {
      active = false;
      off?.();
    };
  }, [reload]);
  const backup = (folder: string) =>
    void operation.run(
      () => api('createBackup', { folder }),
      (result) => {
        setLastFile(result.path);
        reload();
        toast.success('Respaldo completo guardado y verificado.');
      },
    );
  const chooseBackup = () =>
    void operation.run(
      async () => {
        const folder = await api('chooseBackupFolder');
        if (!folder) return null;
        return api('createBackup', { folder });
      },
      (result) => {
        if (result) {
          setLastFile(result.path);
          reload();
          toast.success('Copia lista para llevar al otro equipo.');
        }
      },
    );
  if (!overview)
    return (
      <>
        <Failure error={load.error} retry={reload} />
        {!load.error && <Busy>Comprobando tu almacenamiento…</Busy>}
      </>
    );
  const latest = overview.history.find((r) => r.successful);
  return (
    <main className="page-scroll data-manager">
      <PageTitle
        eyebrow="CONFIGURACIÓN / ADMINISTRACIÓN DE DATOS"
        title="Tu negocio, bien resguardado."
        description="Información local, respaldos verificados y control de tu almacenamiento."
        actions={
          <Button variant="outline" onClick={reload}>
            <RefreshCw size={16} />
            Actualizar
          </Button>
        }
      />
      <Failure error={operation.error} />
      <div className="data-metrics">
        <div>
          <span>
            <Database size={17} />
            Base local
          </span>
          <strong>SQLite · versión {overview.schemaVersion}</strong>
          <small>{bytes(overview.bytes)} de información</small>
        </div>
        <div>
          <span>
            <ShieldCheck size={17} />
            Integridad
          </span>
          <strong>{overview.lastIntegrity ? 'Comprobada' : 'Pendiente'}</strong>
          <small>{date(overview.lastIntegrity)}</small>
        </div>
        <div>
          <span>
            <Clock3 size={17} />
            Último respaldo
          </span>
          <strong>{latest ? date(latest.createdAt) : 'Crea tu primera copia'}</strong>
          <small>
            {overview.settings.automaticBackups
              ? `Automático cada ${overview.settings.backupIntervalHours} h, con la aplicación abierta`
              : 'Respaldos automáticos desactivados'}
          </small>
        </div>
      </div>
      <section className="panel data-location">
        <div>
          <Database size={23} />
          <div>
            <h2>Ubicación de la base</h2>
            <p className="file-location">{overview.path}</p>
            <small>{bytes(overview.freeBytes)} disponibles en este disco</small>
          </div>
        </div>
        <Button
          variant="outline"
          disabled={operation.busy}
          onClick={() =>
            void operation.run(
              () => api('verifyDatabase'),
              () => {
                reload();
                toast.success('Estructura, relaciones e información verificadas.');
              },
            )
          }
        >
          <ShieldCheck size={17} />
          Comprobar ahora
        </Button>
      </section>
      <div className="data-actions-grid">
        <section className="panel data-feature">
          <HardDriveDownload size={26} />
          <h2>Respaldos completos</h2>
          <p>Incluyen ventas, inventario, usuarios, logotipo y preferencias personales.</p>
          <Button
            disabled={operation.busy}
            onClick={() => backup(overview.settings.backupDirectory)}
          >
            <HardDriveDownload size={17} />
            {operation.busy ? 'Procesando…' : 'Crear respaldo ahora'}
          </Button>
          <Button variant="outline" disabled={operation.busy} onClick={chooseBackup}>
            Guardar copia en otra carpeta
          </Button>
          <button className="text-action" onClick={() => setOptions(true)}>
            Configurar carpeta y frecuencia <ArrowRight size={14} />
          </button>
          <small className="file-location">{overview.settings.backupDirectory}</small>
        </section>
        <section className="panel data-feature">
          <MonitorUp size={26} />
          <h2>Recuperación y traslado</h2>
          <p>Restaura un respaldo o importa una base compatible conservando el archivo original.</p>
          <Button variant="outline" onClick={() => setRestore(true)}>
            <FolderOpen size={17} />
            Restaurar / seleccionar base
          </Button>
          <Button
            variant="outline"
            onClick={() => {
              setLastFile('');
              setTransfer(true);
            }}
          >
            <MonitorUp size={17} />
            Cambiar de computadora
          </Button>
          <button className="text-action" onClick={() => setFresh(true)}>
            <FilePlus2 size={15} />
            Crear una base nueva
          </button>
        </section>
      </div>
      {lastFile && !transfer && (
        <div className="notice backup-success">
          <Check size={19} />
          <div>
            <strong>
              {lastFile.endsWith('.donovanbackup') ? 'Copia verificada' : 'Catálogo exportado'}
            </strong>
            <p className="file-location">{lastFile}</p>
          </div>
        </div>
      )}
      <section className="panel">
        <div className="section-heading">
          <div>
            <h2>Historial de respaldos</h2>
            <p>Últimas 100 operaciones de esta base.</p>
          </div>
        </div>
        <DataGrid
          rows={overview.history}
          caption="Historial de respaldos"
          emptyTitle="Aún no se ha creado un respaldo."
          columns={[
            {
              id: 'date',
              label: 'Fecha',
              value: (r) => r.createdAt,
              render: (r) => date(r.createdAt),
            },
            { id: 'kind', label: 'Tipo', value: (r) => r.kind },
            {
              id: 'status',
              label: 'Resultado',
              value: (r) => (r.successful ? 1 : 0),
              render: (r) => (
                <span className={`stock-badge ${r.successful ? 'normal' : 'empty'}`}>
                  {r.successful ? 'Verificado' : 'Requiere atención'}
                </span>
              ),
            },
            {
              id: 'bytes',
              label: 'Tamaño',
              value: (r) => r.fileBytes,
              render: (r) => bytes(r.fileBytes),
              align: 'right',
            },
            {
              id: 'path',
              label: 'Archivo / detalle',
              value: (r) => r.filePath,
              render: (r) => (
                <div className="file-location">{r.successful ? r.filePath : r.result}</div>
              ),
            },
          ]}
        />
      </section>
      <section className="panel diagnostic-summary">
        <div>
          <h2>Productos y archivos de Excel</h2>
          <p>Importa con vista previa o guarda una copia del catálogo.</p>
        </div>
        <div className="page-actions">
          <Button
            variant="outline"
            disabled={operation.busy}
            onClick={() =>
              void operation.run(
                () => api('exportProducts', { template: false }),
                (result) => {
                  if (result) {
                    setLastFile(result.path);
                    toast.success('Catálogo exportado.');
                  }
                },
              )
            }
          >
            Exportar catálogo
          </Button>
          <Button onClick={() => setImporting(true)}>Importar productos</Button>
        </div>
      </section>
      {importing && <ImportProducts onClose={() => setImporting(false)} onImported={reload} />}
      <section className="panel diagnostic-summary">
        <div>
          <h2>Diagnóstico y compatibilidad</h2>
          <p>Consulta las migraciones y los incidentes de conexión o almacenamiento.</p>
        </div>
        <Button
          variant="outline"
          onClick={() => void operation.run(() => api('diagnostics'), setDiagnostics)}
        >
          Ver diagnóstico
        </Button>
      </section>
      <RestoreDialog
        open={restore}
        onClose={() => setRestore(false)}
        data={data}
        onRestored={onChanged}
        hasDraft={hasDraft}
      />
      {options && (
        <BackupOptions
          overview={overview}
          onClose={() => setOptions(false)}
          onSaved={setOverview}
        />
      )}
      <NewDatabase
        open={fresh}
        hasDraft={hasDraft}
        onClose={() => setFresh(false)}
        onChanged={onChanged}
      />
      <Modal
        open={transfer}
        onClose={() => {
          if (!operation.busy) setTransfer(false);
        }}
        title="Lleva tu negocio a otro equipo"
        description="Usa el mismo instalador de Windows y una copia final verificada."
        className="data-dialog"
      >
        <ol className="transfer-steps">
          <li>
            <b>1</b>
            <div>
              <strong>Termina las operaciones de este equipo.</strong>
              <p>
                Suspende o termina las ventas en preparación. Después del respaldo final, continúa
                trabajando únicamente en el nuevo equipo.
              </p>
            </div>
          </li>
          <li>
            <b>2</b>
            <div>
              <strong>Guarda una copia en tu USB o disco externo.</strong>
              <p>Elige su carpeta y espera a que aparezca la confirmación de integridad.</p>
              <Button onClick={chooseBackup} disabled={operation.busy}>
                <HardDriveDownload size={17} />
                {operation.busy ? 'Preparando la copia…' : 'Elegir destino y respaldar'}
              </Button>
              {lastFile && (
                <p className="file-location">
                  <Check size={16} /> {lastFile}
                </p>
              )}
            </div>
          </li>
          <li>
            <b>3</b>
            <div>
              <strong>Instala Donovan POS Pro en la otra computadora.</strong>
              <p>
                En la bienvenida, elige «Restaurar respaldo», selecciona esta copia y autentícate
                con el administrador del negocio. Tus contraseñas se conservan.
              </p>
            </div>
          </li>
          <li>
            <b>4</b>
            <div>
              <strong>Verifica tu negocio y selecciona la impresora.</strong>
              <p>
                Revisa las existencias y la última venta, configura la ticketera y crea un respaldo
                en el nuevo equipo.
              </p>
            </div>
          </li>
        </ol>
        <Failure error={operation.error} />
        <Button variant="outline" onClick={() => setTransfer(false)} disabled={operation.busy}>
          Entendido
        </Button>
      </Modal>
      <Modal
        open={diagnostics !== null}
        onClose={() => setDiagnostics(null)}
        title="Diagnóstico local"
        description="Referencias de los últimos incidentes, sin contraseñas ni datos de cobro."
        className="wide-dialog"
      >
        <h3>Migraciones aplicadas</h3>
        <div className="migration-list">
          {overview.migrations.map((m) => (
            <span key={m.version}>
              <Check size={15} /> v{m.version} · {m.description} · {date(m.appliedAt)}
            </span>
          ))}
        </div>
        <div className="notice">
          La administración cotidiana se realiza aquí. SQLiteStudio puede utilizarse para
          diagnóstico técnico; cierra Donovan POS Pro antes de hacer modificaciones directas.
        </div>
        {diagnostics?.length ? (
          <div className="diagnostic-list">
            {diagnostics.map((d, i) => (
              <article key={`${d.incidentId}-${i}`}>
                <strong>{d.message || 'Incidente registrado'}</strong>
                <p>
                  {date(d.time)} · {d.code} · {d.operation}
                </p>
                <small>Referencia {d.incidentId}</small>
              </article>
            ))}
          </div>
        ) : (
          <Empty title="No hay incidentes recientes registrados." />
        )}
      </Modal>
    </main>
  );
}
function BackupOptions({
  overview,
  onClose,
  onSaved,
}: {
  overview: DataOverview;
  onClose: () => void;
  onSaved: (d: DataOverview) => void;
}) {
  const [folder, setFolder] = useState(overview.settings.backupDirectory);
  const [enabled, setEnabled] = useState(overview.settings.automaticBackups);
  const [hours, setHours] = useState(overview.settings.backupIntervalHours);
  const [password, setPassword] = useState('');
  const operation = useOperation();
  return (
    <Modal
      open
      onClose={() => {
        if (!operation.busy) onClose();
      }}
      title="Configurar respaldos"
      description="Se ejecutan automáticamente mientras Donovan POS Pro está abierto."
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void operation.run(
            () => api('saveBackupSettings', { folder, enabled, hours, password }),
            (d) => {
              setPassword('');
              onSaved(d);
              onClose();
              toast.success('Configuración de respaldos guardada.');
            },
          );
        }}
      >
        <Failure error={operation.error} />
        <Field label="Carpeta de destino">
          <span className="file-location">{folder}</span>
          <Button
            type="button"
            variant="outline"
            disabled={operation.busy}
            onClick={() =>
              void operation.run(
                () => api('chooseBackupFolder'),
                (p) => {
                  if (p) setFolder(p);
                },
              )
            }
          >
            <FolderOpen size={16} />
            Elegir carpeta
          </Button>
        </Field>
        <label className="check-row">
          <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} />
          Crear respaldos automáticos
        </label>
        <Field
          label="Frecuencia en horas"
          hint="De 1 a 168 horas. Una unidad desconectada se reporta en el diagnóstico."
        >
          <input
            type="number"
            min={1}
            max={168}
            step={1}
            required
            value={hours}
            onChange={(e) => setHours(Number(e.target.value))}
          />
        </Field>
        <Field label="Tu contraseña de administrador">
          <input
            type="password"
            autoComplete="current-password"
            value={password}
            maxLength={128}
            required
            onChange={(e) => setPassword(e.target.value)}
          />
        </Field>
        <div className="dialog-actions">
          <Button type="button" variant="outline" disabled={operation.busy} onClick={onClose}>
            Cancelar
          </Button>
          <Button type="submit" disabled={operation.busy}>
            Guardar configuración
          </Button>
        </div>
      </form>
    </Modal>
  );
}
function NewDatabase({
  open,
  onClose,
  onChanged,
  hasDraft,
}: {
  open: boolean;
  onClose: () => void;
  onChanged: (d: Bootstrap) => void;
  hasDraft: boolean;
}) {
  const operation = useOperation();
  const [password, setPassword] = useState('');
  const [agreed, setAgreed] = useState(false);
  return (
    <Modal
      open={open}
      onClose={() => {
        if (!operation.busy) {
          setPassword('');
          setAgreed(false);
          onClose();
        }
      }}
      title="Crear otra base local"
      description="Se creará una base vacía y volverás a la configuración inicial."
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (agreed)
            void operation.run(
              () => api('newDatabase', { password }),
              (d) => {
                setPassword('');
                setAgreed(false);
                onChanged(d);
                onClose();
              },
            );
        }}
      >
        <Failure error={operation.error} />
        <div className="notice">
          Primero se guardará un respaldo preventivo completo. La base anterior permanecerá en su
          carpeta y podrás recuperarla seleccionándola desde Administración de datos.
        </div>
        {hasDraft && (
          <div className="notice warning">
            Los carritos en preparación que no hayas suspendido se descartarán al cambiar de base.
          </div>
        )}
        <Field label="Tu contraseña de administrador">
          <input
            type="password"
            value={password}
            maxLength={128}
            autoComplete="current-password"
            required
            onChange={(e) => setPassword(e.target.value)}
          />
        </Field>
        <label className="check-row">
          <input type="checkbox" checked={agreed} onChange={(e) => setAgreed(e.target.checked)} />
          Quiero configurar una base nueva para esta instalación.
        </label>
        <div className="dialog-actions">
          <Button type="button" variant="outline" disabled={operation.busy} onClick={onClose}>
            Cancelar
          </Button>
          <Button type="submit" disabled={operation.busy || !agreed}>
            <FilePlus2 size={16} />
            {operation.busy ? 'Preparando la base…' : 'Crear base nueva'}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
