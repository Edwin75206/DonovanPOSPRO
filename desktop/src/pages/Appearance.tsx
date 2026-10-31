import { useEffect, useMemo, useState } from 'react';
import {
  Check,
  ImagePlus,
  Monitor,
  Sun,
  Moon,
  RotateCcw,
  Type,
  PanelLeft,
  Palette,
  Move,
  Save,
  X,
} from 'lucide-react';
import { toast } from 'sonner';
import { api, type Bootstrap } from '@/lib/api';
import {
  defaultAppearance,
  presets,
  themeTokens,
  useVisual,
  type AppearanceConfig,
  type AppearanceState,
} from '@/lib/appearance';
import { Button, Field, Failure, PageTitle, useOperation } from '@/components/common';
import { useLeaveGuard } from '@/lib/leaveGuard';
const colors = [
  '#c82d2b',
  '#2565a9',
  '#e4bb35',
  '#247354',
  '#c26a20',
  '#087b83',
  '#7954aa',
  '#414b52',
];
const colorNames = [
  'Rojo',
  'Azul',
  'Amarillo',
  'Verde',
  'Naranja',
  'Turquesa',
  'Violeta',
  'Neutro',
];
function Choice<T extends string>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
  label: string;
}) {
  return (
    <div className="choice-group" role="group" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          aria-pressed={value === o.value}
          className={value === o.value ? 'selected' : ''}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
export function Appearance({
  data,
  onSaved,
}: {
  data: Bootstrap;
  onSaved: (appearance: AppearanceState) => void;
}) {
  const source = useMemo(
    () => data.appearance ?? defaultAppearance(data.actor?.role),
    [data.appearance, data.actor?.role],
  );
  const [draft, setDraft] = useState(source);
  const [previewDark, setPreviewDark] = useState(false);
  const visual = useVisual();
  const op = useOperation();
  useEffect(() => setDraft(source), [source]);
  useEffect(() => {
    visual.preview(draft);
  }, [draft, visual.preview]);
  useEffect(() => () => visual.preview(null), [visual.preview]);
  const change = <K extends keyof AppearanceConfig>(key: K, value: AppearanceConfig[K]) =>
    setDraft((s) => ({ ...s, config: { ...s.config, [key]: value } }));
  const c = draft.config;
  const dirty =
    JSON.stringify(c) !== JSON.stringify(source.config) ||
    draft.backgroundData !== source.backgroundData;
  const preview = themeTokens(c, previewDark);
  const leaveGuard = useLeaveGuard(dirty);
  const preset = presets.find((p) => p.id === c.preset) ?? presets[0];
  const cancel = () => {
    setDraft(source);
    visual.preview(null);
    op.setError(null);
  };
  const save = () =>
    op.run(
      () =>
        api('saveAppearance', {
          config: c,
          backgroundAction:
            draft.backgroundData === source.backgroundData
              ? 'keep'
              : draft.backgroundData
                ? 'replace'
                : 'remove',
          backgroundData: draft.backgroundData,
        }),
      (saved) => {
        onSaved(saved);
        setDraft(saved);
        visual.preview(null);
        toast.success('La apariencia se guardó para tu cuenta.');
      },
    );
  return (
    <main className="page-scroll appearance-page">
      {leaveGuard}
      <PageTitle
        eyebrow="CONFIGURACIÓN PERSONAL"
        title="Apariencia y personalización"
        description="Elige cómo quieres trabajar. Estos cambios son solo para tu cuenta."
      />
      <div className="appearance-layout">
        <div className="appearance-options">
          <section className="panel appearance-section">
            <div className="section-label">
              <Palette size={18} />
              <div>
                <h2>Una combinación que vaya contigo</h2>
                <p>Temas completos, con superficies y gráficas coordinadas.</p>
              </div>
            </div>
            <div className="theme-grid">
              {presets.map((p) => {
                const palette = themeTokens(
                  { ...c, preset: p.id, primary: null },
                  p.id === 'night',
                );
                return (
                  <button
                    key={p.id}
                    className={`theme-tile ${c.preset === p.id ? 'selected' : ''}`}
                    onClick={() =>
                      setDraft((s) => ({
                        ...s,
                        config: {
                          ...s.config,
                          preset: p.id,
                          primary: null,
                          ...(p.id === 'night'
                            ? { mode: 'dark' as const }
                            : p.id === 'minimal'
                              ? { mode: 'light' as const }
                              : {}),
                        },
                      }))
                    }
                    aria-pressed={c.preset === p.id}
                  >
                    <div className="mini-theme" style={{ background: palette['--background'] }}>
                      <i style={{ background: palette['--hero'] }} />
                      <div>
                        <span style={{ background: palette['--primary'] }} />
                        <span style={{ background: palette['--surface'] }} />
                        <b style={{ background: palette['--surface'] }}>
                          <em style={{ background: palette['--primary'] }} />
                          <em style={{ background: palette['--chart-2'] }} />
                          <em style={{ background: palette['--yellow'] }} />
                        </b>
                      </div>
                    </div>
                    <span>
                      {p.id === 'client' ? (data.settings?.storeName ?? p.name) : p.name}
                      {c.preset === p.id && <Check size={14} />}
                    </span>
                  </button>
                );
              })}
            </div>
            <div className="setting-row">
              <div>
                <strong>Modo de color</strong>
                <small>Automático sigue los cambios de Windows.</small>
              </div>
              <Choice
                label="Modo de color"
                value={c.mode}
                onChange={(v) => change('mode', v)}
                options={[
                  { value: 'light', label: 'Claro' },
                  { value: 'dark', label: 'Oscuro' },
                  { value: 'system', label: 'Automático' },
                ]}
              />
            </div>
            <div className="setting-row vertical">
              <div>
                <strong>Tu color principal</strong>
                <small>El contraste del texto se ajusta automáticamente.</small>
              </div>
              <div className="color-choices">
                {colors.map((color, i) => (
                  <button
                    key={color}
                    aria-label={colorNames[i]}
                    aria-pressed={(c.primary ?? preset.accent) === color}
                    className="color-swatch"
                    style={{ background: color }}
                    onClick={() => change('primary', color)}
                  >
                    {(c.primary ?? preset.accent) === color && (
                      <Check
                        size={17}
                        style={{
                          color: themeTokens({ ...c, primary: color }, false)[
                            '--primary-foreground'
                          ],
                        }}
                      />
                    )}
                  </button>
                ))}
                <input
                  aria-label="Selector de color personalizado"
                  type="color"
                  value={
                    /^#[a-f\d]{6}$/i.test(c.primary ?? preset.accent)
                      ? (c.primary ?? preset.accent)
                      : preset.accent
                  }
                  onChange={(e) => change('primary', e.target.value)}
                />
                <input
                  aria-label="Color hexadecimal"
                  className="hex-input"
                  value={c.primary ?? preset.accent}
                  maxLength={7}
                  onChange={(e) => change('primary', e.target.value)}
                />
              </div>
            </div>
          </section>
          <section className="panel appearance-section">
            <div className="section-label">
              <Type size={19} />
              <div>
                <h2>Lee y trabaja a tu ritmo</h2>
                <p>El tamaño de letra se aplica a toda la interfaz.</p>
              </div>
            </div>
            <div className="font-scale-controls">
              <span>Aa</span>
              <input
                aria-label="Tamaño de letra"
                type="range"
                min={90}
                max={150}
                step={5}
                value={c.fontScale}
                onChange={(e) => change('fontScale', Number(e.target.value))}
              />
              <strong>{c.fontScale}%</strong>
            </div>
            <Choice
              label="Tamaños de letra sugeridos"
              value={String(c.fontScale)}
              onChange={(v) => change('fontScale', Number(v))}
              options={[
                { value: '90', label: 'Compacto' },
                { value: '100', label: 'Normal' },
                { value: '110', label: 'Cómodo' },
                { value: '125', label: 'Grande' },
                { value: '150', label: 'Extra grande' },
              ]}
            />
            <div className="setting-row">
              <div>
                <strong>Densidad</strong>
                <small>Ajusta espacio y altura de filas.</small>
              </div>
              <Choice
                label="Densidad de la interfaz"
                value={c.density}
                onChange={(v) => change('density', v)}
                options={[
                  { value: 'compact', label: 'Compacta' },
                  { value: 'standard', label: 'Estándar' },
                  { value: 'comfortable', label: 'Cómoda' },
                ]}
              />
            </div>
            <div className="setting-row">
              <div>
                <strong>Navegación</strong>
                <small>Siempre disponible por teclado.</small>
              </div>
              <Choice
                label="Navegación"
                value={c.sidebar}
                onChange={(v) => change('sidebar', v)}
                options={[
                  { value: 'expanded', label: 'Expandida' },
                  { value: 'compact', label: 'Compacta' },
                ]}
              />
            </div>
            <div className="setting-row">
              <div>
                <strong>Animaciones</strong>
                <small>Se respeta el movimiento reducido de Windows.</small>
              </div>
              <Choice
                label="Nivel de animaciones"
                value={c.animations}
                onChange={(v) => change('animations', v)}
                options={[
                  { value: 'none', label: 'Desactivadas' },
                  { value: 'subtle', label: 'Suaves' },
                  { value: 'full', label: 'Completas' },
                ]}
              />
            </div>
          </section>
          <section className="panel appearance-section">
            <div className="section-label">
              <ImagePlus size={19} />
              <div>
                <h2>Un fondo para tu espacio</h2>
                <p>Productos, precios y formularios conservan superficies legibles.</p>
              </div>
            </div>
            <Field label="Estilo del fondo">
              <select value={c.background} onChange={(e) => change('background', e.target.value)}>
                <option value="theme">Propio del tema</option>
                <option value="light">Claro</option>
                <option value="dark">Oscuro</option>
                <option value="gray">Gris suave</option>
                <option value="tinted">Tenue del tema</option>
                <option value="gradient">Degradado discreto</option>
                <option value="solid">Color sólido</option>
                <option value="image">Imagen personal</option>
              </select>
            </Field>
            {c.background === 'solid' && (
              <Field label="Color del fondo">
                <input
                  type="color"
                  value={c.backgroundColor}
                  onChange={(e) => change('backgroundColor', e.target.value)}
                />
              </Field>
            )}
            {c.background === 'image' && (
              <>
                <div className="background-upload">
                  <Button
                    variant="outline"
                    disabled={op.busy}
                    onClick={() =>
                      void op.run(
                        () => api('chooseAppearanceImage'),
                        (result) => {
                          if (result) setDraft((s) => ({ ...s, backgroundData: result.data }));
                        },
                      )
                    }
                  >
                    <ImagePlus size={17} />
                    {draft.backgroundData ? 'Cambiar imagen' : 'Elegir imagen'}
                  </Button>
                  {draft.backgroundData && (
                    <Button
                      variant="ghost"
                      onClick={() => setDraft((s) => ({ ...s, backgroundData: null }))}
                    >
                      Quitar
                    </Button>
                  )}
                  <small>PNG o JPEG · hasta 5 MB</small>
                </div>
                {draft.backgroundData && (
                  <>
                    <Field
                      label={`Intensidad · ${c.imageOpacity}%`}
                      hint="Una capa protectora mantiene el contraste."
                    >
                      <input
                        type="range"
                        min={0}
                        max={40}
                        value={c.imageOpacity}
                        onChange={(e) => change('imageOpacity', Number(e.target.value))}
                      />
                    </Field>
                    <Field label="Encuadre">
                      <select
                        value={c.imageFit}
                        onChange={(e) => change('imageFit', e.target.value as 'cover' | 'contain')}
                      >
                        <option value="cover">Llenar el fondo</option>
                        <option value="contain">Mostrar imagen completa</option>
                      </select>
                    </Field>
                    <div className="form-grid">
                      <Field label="Posición horizontal">
                        <input
                          type="range"
                          min={0}
                          max={100}
                          value={c.imageX}
                          onChange={(e) => change('imageX', Number(e.target.value))}
                        />
                      </Field>
                      <Field label="Posición vertical">
                        <input
                          type="range"
                          min={0}
                          max={100}
                          value={c.imageY}
                          onChange={(e) => change('imageY', Number(e.target.value))}
                        />
                      </Field>
                    </div>
                  </>
                )}
              </>
            )}
          </section>
        </div>
        <aside className="appearance-preview">
          <div className="preview-heading">
            <span>VISTA PREVIA</span>
            <div>
              <button
                aria-label="Vista previa clara"
                aria-pressed={!previewDark}
                onClick={() => setPreviewDark(false)}
              >
                <Sun size={16} />
              </button>
              <button
                aria-label="Vista previa oscura"
                aria-pressed={previewDark}
                onClick={() => setPreviewDark(true)}
              >
                <Moon size={16} />
              </button>
            </div>
          </div>
          <div
            className="preview-surface"
            style={
              {
                ...preview,
                background: preview['--background'],
                color: preview['--foreground'],
              } as React.CSSProperties
            }
          >
            <div className="preview-title">
              <span>Resumen del negocio</span>
              <span className="avatar">AD</span>
            </div>
            <div className="preview-kpis">
              <div>
                <small>Ventas del día</small>
                <strong>$12,450.00</strong>
                <span>18 operaciones</span>
              </div>
              <div>
                <small>En caja</small>
                <strong>$4,200.00</strong>
                <span>Abierta</span>
              </div>
            </div>
            <div className="preview-bars" aria-hidden="true">
              {[25, 49, 38, 64, 53, 79, 68].map((height, i) => (
                <i
                  key={i}
                  style={{
                    height: `${height}%`,
                    background: i === 5 ? preview['--chart-2'] : preview['--primary'],
                  }}
                />
              ))}
            </div>
            <div className="preview-table">
              <div>
                <b>Producto</b>
                <b>Precio</b>
              </div>
              <div>
                <span>Cemento gris</span>
                <span>$235.00</span>
              </div>
              <div>
                <span>Cable calibre 12</span>
                <span>$18.50</span>
              </div>
            </div>
            <label className="field">
              <span>Buscar producto</span>
              <input value="Materiales para tu obra" readOnly tabIndex={-1} />
            </label>
            <div className="preview-buttons">
              <span
                style={{ background: preview['--primary'], color: preview['--primary-foreground'] }}
              >
                Cobrar venta
              </span>
              <span style={{ border: `1px solid ${preview['--border']}` }}>Cotizar</span>
            </div>
          </div>
          <p>
            Datos ilustrativos. Los cambios también se muestran en tu aplicación mientras los
            pruebas.
          </p>
          <div className="notice">
            <Palette size={19} />
            <p>La identidad comercial y los tickets no cambian al personalizar tu pantalla.</p>
          </div>
        </aside>
      </div>
      <div className="appearance-save">
        <div>
          <strong>{dirty ? 'Así se verá tu espacio.' : 'Tu apariencia está guardada.'}</strong>
          <small>
            {dirty
              ? 'Aplica para conservar los cambios al volver a entrar.'
              : 'Solo afecta a ' + data.actor?.name + '.'}
          </small>
        </div>
        <div className="appearance-actions">
          <Button
            variant="ghost"
            onClick={() => {
              setDraft(defaultAppearance(data.actor?.role));
              op.setError(null);
            }}
          >
            <RotateCcw size={15} />
            Restablecer apariencia
          </Button>
          <Button variant="outline" onClick={cancel} disabled={op.busy}>
            Cancelar
          </Button>
          <Button onClick={() => void save()} disabled={!dirty || op.busy}>
            <Save size={16} />
            {op.busy ? 'Guardando…' : 'Aplicar cambios'}
          </Button>
        </div>
        <Failure error={op.error} />
      </div>
    </main>
  );
}
