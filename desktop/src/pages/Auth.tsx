import { useState } from 'react';
import { motion } from 'motion/react';
import {
  ArrowRight,
  Eye,
  EyeOff,
  ShieldCheck,
  Store,
  LockKeyhole,
  Check,
  ChevronLeft,
  HardDrive,
  ReceiptText,
} from 'lucide-react';
import { api, type Bootstrap, type Setup } from '@/lib/api';
import { Button, Field, Failure, useOperation } from '@/components/common';
import defaultLogo from '../../../assets/branding/customer-logo-transparent.png';
export function Logo({ data, className = '' }: { data?: Bootstrap; className?: string }) {
  const logo = data?.settings
    ? data.settings.logoData
      ? `data:image/${data.settings.logoData.startsWith('/9j') ? 'jpeg' : 'png'};base64,${data.settings.logoData}`
      : null
    : defaultLogo;
  return logo ? (
    <img
      className={`business-logo ${className}`}
      src={logo}
      alt={`Logotipo de ${data?.settings?.storeName ?? data?.profile?.storeName ?? 'tu negocio'}`}
    />
  ) : (
    <div className={`brand-monogram ${className}`}>D</div>
  );
}
export function Auth({
  data,
  onSession,
  onRestore,
}: {
  data: Bootstrap;
  onSession: (value: Bootstrap) => void;
  onRestore: () => void;
}) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [show, setShow] = useState(false);
  const op = useOperation();
  const name = data.settings?.storeName ?? data.profile?.storeName ?? 'Tu negocio';
  return (
    <main className="auth-layout">
      <section className="auth-story">
        <div className="publisher">
          <span className="donovan-mark">D</span> DONOVAN <span>POS PRO</span>
        </div>
        <div className="auth-business">
          <Logo data={data} />
          <span className="eyebrow">TU NEGOCIO, EN BUENAS MANOS</span>
          <h1>
            Tu mostrador.
            <br />
            Todo bajo control.
          </h1>
          <p>
            Una forma más sencilla de vender,
            <br />
            organizar y seguir construyendo.
          </p>
        </div>
        <div className="auth-trust">
          <HardDrive size={17} />
          <span>Información local · Disponible sin internet</span>
        </div>
      </section>
      <motion.section
        className="auth-form-side"
        initial={{ opacity: 0, x: 8 }}
        animate={{ opacity: 1, x: 0 }}
        transition={{ duration: 0.2 }}
      >
        <div className="auth-form">
          <div className="store-label">
            <Store size={17} />
            {name}
          </div>
          <span className="eyebrow">
            {data.locked ? 'PANTALLA BLOQUEADA' : 'BIENVENIDO DE NUEVO'}
          </span>
          <h2>{data.locked ? 'Continúa tu jornada.' : 'Vamos a trabajar.'}</h2>
          <p>
            {data.locked
              ? 'Ingresa tu contraseña para desbloquear.'
              : 'Ingresa con tu cuenta para abrir tu espacio de trabajo.'}
          </p>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void op.run(
                () =>
                  data.locked ? api('unlock', { password }) : api('login', { username, password }),
                (v) => {
                  setPassword('');
                  onSession(v);
                },
              );
            }}
          >
            {!data.locked && (
              <Field label="Usuario">
                <input
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  autoFocus
                  autoComplete="username"
                  placeholder="Tu nombre de usuario"
                  required
                  maxLength={50}
                />
              </Field>
            )}
            <Field label="Contraseña">
              <div className="password-field">
                <input
                  type={show ? 'text' : 'password'}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  autoComplete="current-password"
                  placeholder="Ingresa tu contraseña"
                  required
                  maxLength={128}
                  autoFocus={data.locked}
                />
                <button
                  type="button"
                  className="icon-button"
                  aria-label={show ? 'Ocultar contraseña' : 'Mostrar contraseña'}
                  onClick={() => setShow(!show)}
                >
                  {show ? <EyeOff size={18} /> : <Eye size={18} />}
                </button>
              </div>
            </Field>
            <Failure error={op.error} />
            <Button type="submit" className="wide" disabled={op.busy}>
              {op.busy ? 'Verificando…' : data.locked ? 'Desbloquear' : 'Entrar a mi espacio'}
              <ArrowRight size={17} />
            </Button>
          </form>
          {data.locked && (
            <button
              className="text-button"
              onClick={() => void op.run(() => api('logout'), onSession)}
            >
              Cambiar de usuario
            </button>
          )}
          <div className="session-note">
            <ShieldCheck size={18} />
            <span>
              Tu sesión se conserva durante 24 horas.
              <br />
              Puedes bloquear la pantalla al alejarte.
            </span>
          </div>
          {data.demo && (
            <div className="demo-entry">
              <b>DEMOSTRACIÓN AISLADA</b>
              <p>Explora con datos de ejemplo.</p>
              <Button
                variant="outline"
                onClick={() => void op.run(() => api('demoLogin', { role: 0 }), onSession)}
              >
                Administrador
              </Button>
              <Button
                variant="outline"
                onClick={() => void op.run(() => api('demoLogin', { role: 1 }), onSession)}
              >
                Negociante Uno
              </Button>
            </div>
          )}
          <button className="text-button secondary" onClick={onRestore}>
            Recuperar información de mi negocio
          </button>
        </div>
        <footer>
          Donovan Systems <span>Donovan POS Pro · {data.version}</span>
        </footer>
      </motion.section>
    </main>
  );
}
export function Welcome({
  data,
  onSession,
  onRestore,
}: {
  data: Bootstrap;
  onSession: (value: Bootstrap) => void;
  onRestore: () => void;
}) {
  const [step, setStep] = useState(0);
  const op = useOperation();
  const [form, setForm] = useState<Setup>({
    storeName: data.profile?.storeName ?? '',
    adminUsername: 'administrador',
    adminName: 'Administrador',
    adminPassword: '',
    sellerUsername: 'negociante.uno',
    sellerName: 'Negociante Uno',
    sellerPassword: '',
    address: '',
    phone: '',
    taxId: '',
    ticketWidthMm: 80,
  });
  const field = (key: keyof Setup, value: string | number) =>
    setForm((v) => ({ ...v, [key]: value }));
  const steps = ['Tu negocio', 'Tus cuentas', 'Tus tickets', 'Todo listo'];
  const submit = async () => {
    await api('setup', form);
    return api('login', { username: form.adminUsername, password: form.adminPassword });
  };
  return (
    <main className="setup-layout">
      <aside>
        <div className="publisher">
          <span className="donovan-mark">D</span> DONOVAN <span>POS PRO</span>
        </div>
        <h1>
          Un comienzo
          <br />
          bien preparado.
        </h1>
        <p>
          Configura lo esencial una sola vez.
          <br />
          Después, lo importante es tu negocio.
        </p>
        <ol>
          {steps.map((s, i) => (
            <li key={s} className={i === step ? 'current' : i < step ? 'done' : ''}>
              <span>{i < step ? <Check size={16} /> : i + 1}</span>
              {s}
            </li>
          ))}
        </ol>
        <div className="setup-aside-note">
          <ShieldCheck size={22} />
          <p>Tus datos se guardan en este equipo y se conservan al actualizar el programa.</p>
        </div>
      </aside>
      <section>
        <div className="setup-top">
          <span>CONFIGURACIÓN INICIAL</span>
          <button className="text-button" onClick={onRestore}>
            Ya tengo un respaldo <ArrowRight size={14} />
          </button>
        </div>
        <motion.form
          key={step}
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.18 }}
          onSubmit={(e) => {
            e.preventDefault();
            if (step < 3) setStep(step + 1);
            else void op.run(submit, onSession);
          }}
        >
          <span className="eyebrow">PASO {step + 1} DE 4</span>
          <h2>
            {
              [
                'Démosle su identidad.',
                'Un acceso para cada persona.',
                'Tickets con tu nombre.',
                'Listo para abrir el mostrador.',
              ][step]
            }
          </h2>
          <p className="subtle">
            {
              [
                'Los puedes cambiar después desde Configuración.',
                'El administrador controla el negocio. El vendedor atiende el mostrador.',
                'Elige el formato de tu impresora. También podrás guardar documentos.',
                'Revisa los datos antes de empezar. Tu catálogo comienza vacío.',
              ][step]
            }
          </p>
          {step === 0 && (
            <>
              <div className="setup-brand">
                <Logo data={data} />
                <span>
                  Identidad comercial del cliente
                  <br />
                  <small>Logotipo del negocio</small>
                </span>
              </div>
              <Field label="Nombre comercial">
                <input
                  value={form.storeName}
                  onChange={(e) => field('storeName', e.target.value)}
                  required
                  maxLength={150}
                />
              </Field>
              <Field label="Dirección">
                <input
                  value={form.address}
                  onChange={(e) => field('address', e.target.value)}
                  maxLength={200}
                />
              </Field>
              <div className="form-grid">
                <Field label="Teléfono">
                  <input
                    value={form.phone}
                    onChange={(e) => field('phone', e.target.value)}
                    maxLength={30}
                  />
                </Field>
                <Field label="RFC (opcional)">
                  <input
                    value={form.taxId}
                    onChange={(e) => field('taxId', e.target.value)}
                    maxLength={13}
                  />
                </Field>
              </div>
            </>
          )}
          {step === 1 && (
            <div className="account-setup">
              {(['admin', 'seller'] as const).map((kind) => (
                <section key={kind}>
                  <div className="account-label">
                    {kind === 'admin' ? <ShieldCheck size={18} /> : <Store size={18} />}
                    <strong>{kind === 'admin' ? 'Administrador' : 'Vendedor'}</strong>
                  </div>
                  <Field label="Nombre">
                    <input
                      value={form[kind === 'admin' ? 'adminName' : 'sellerName']}
                      onChange={(e) =>
                        field(kind === 'admin' ? 'adminName' : 'sellerName', e.target.value)
                      }
                      required
                      maxLength={100}
                    />
                  </Field>
                  <Field label="Usuario">
                    <input
                      value={form[kind === 'admin' ? 'adminUsername' : 'sellerUsername']}
                      onChange={(e) =>
                        field(kind === 'admin' ? 'adminUsername' : 'sellerUsername', e.target.value)
                      }
                      required
                      maxLength={50}
                      autoComplete="off"
                    />
                  </Field>
                  <Field label="Contraseña" hint="De 10 a 128 caracteres.">
                    <input
                      type="password"
                      value={form[kind === 'admin' ? 'adminPassword' : 'sellerPassword']}
                      onChange={(e) =>
                        field(kind === 'admin' ? 'adminPassword' : 'sellerPassword', e.target.value)
                      }
                      required
                      minLength={10}
                      maxLength={128}
                      autoComplete="new-password"
                    />
                  </Field>
                </section>
              ))}
            </div>
          )}
          {step === 2 && (
            <>
              <div className="paper-options">
                {[80, 58].map((width) => (
                  <button
                    key={width}
                    type="button"
                    className={form.ticketWidthMm === width ? 'selected' : ''}
                    onClick={() => field('ticketWidthMm', width)}
                  >
                    <ReceiptText size={36} />
                    <b>{width} mm</b>
                    <span>
                      {width === 80 ? 'Más espacio para los detalles' : 'Formato compacto'}
                    </span>
                    {form.ticketWidthMm === width && <Check size={17} />}
                  </button>
                ))}
              </div>
              <div className="notice">
                <LockKeyhole size={20} />
                <p>
                  Elige la impresora al imprimir tu primer ticket. Si todavía no tienes una, podrás
                  consultar y guardar tus comprobantes.
                </p>
              </div>
            </>
          )}
          {step === 3 && (
            <div className="setup-summary">
              <Logo data={data} />
              <h3>{form.storeName}</h3>
              <p>{form.address || 'Dirección pendiente de configurar'}</p>
              <dl>
                <div>
                  <dt>Administrador</dt>
                  <dd>{form.adminUsername}</dd>
                </div>
                <div>
                  <dt>Vendedor</dt>
                  <dd>{form.sellerUsername}</dd>
                </div>
                <div>
                  <dt>Ticket</dt>
                  <dd>{form.ticketWidthMm} mm</dd>
                </div>
                <div>
                  <dt>Respaldo automático</dt>
                  <dd>Cada 24 h, mientras esté abierto</dd>
                </div>
              </dl>
            </div>
          )}
          <Failure error={op.error} />
          <div className="setup-actions">
            <Button
              type="button"
              variant="ghost"
              onClick={() => setStep(step - 1)}
              disabled={step === 0 || op.busy}
            >
              <ChevronLeft size={16} />
              Atrás
            </Button>
            <Button type="submit" disabled={op.busy}>
              {op.busy ? 'Preparando…' : step === 3 ? 'Preparar mi negocio' : 'Continuar'}
              <ArrowRight size={17} />
            </Button>
          </div>
        </motion.form>
        <footer>Donovan Systems · Diseñado para tu jornada de trabajo.</footer>
      </section>
    </main>
  );
}
