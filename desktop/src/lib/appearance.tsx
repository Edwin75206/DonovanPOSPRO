import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';
import { MotionConfig } from 'motion/react';
export interface AppearanceConfig {
  mode: 'system' | 'light' | 'dark';
  preset: string;
  primary: string | null;
  fontScale: number;
  density: 'compact' | 'standard' | 'comfortable';
  animations: 'none' | 'subtle' | 'full';
  sidebar: 'expanded' | 'compact';
  background: string;
  backgroundColor: string;
  imageOpacity: number;
  imageX: number;
  imageY: number;
  imageFit: 'cover' | 'contain';
}
export interface AppearanceState {
  config: AppearanceConfig;
  backgroundData: string | null;
}
export const defaultAppearance = (role = 0): AppearanceState => ({
  config: {
    mode: 'system',
    preset: 'client',
    primary: null,
    fontScale: role === 1 ? 110 : 100,
    density: role === 1 ? 'comfortable' : 'standard',
    animations: 'subtle',
    sidebar: role === 1 ? 'compact' : 'expanded',
    background: 'theme',
    backgroundColor: '#f4f3ef',
    imageOpacity: 15,
    imageX: 50,
    imageY: 50,
    imageFit: 'cover',
  },
  backgroundData: null,
});
export const presets = [
  {
    id: 'client',
    name: 'Identidad del negocio',
    accent: '#c82d2b',
    tint: '#5e5647',
    yellow: '#efc84d',
  },
  {
    id: 'executive',
    name: 'Azul ejecutivo',
    accent: '#2565a9',
    tint: '#354d72',
    yellow: '#c7d8f4',
  },
  {
    id: 'forest',
    name: 'Verde profesional',
    accent: '#247354',
    tint: '#3b604d',
    yellow: '#d3dea7',
  },
  {
    id: 'turquoise',
    name: 'Turquesa moderno',
    accent: '#087b83',
    tint: '#325c64',
    yellow: '#ace0d7',
  },
  { id: 'night', name: 'Oscuro elegante', accent: '#b89866', tint: '#4a4557', yellow: '#dac5a0' },
  {
    id: 'minimal',
    name: 'Claro minimalista',
    accent: '#414b52',
    tint: '#64686b',
    yellow: '#deded6',
  },
];
const rgb = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
export function mix(a: string, b: string, part: number) {
  return (
    '#' +
    rgb(a)
      .map((n, i) =>
        Math.round(n * part + rgb(b)[i] * (1 - part))
          .toString(16)
          .padStart(2, '0'),
      )
      .join('')
  );
}
export function contrast(a: string, b: string) {
  const luminance = (c: string) =>
    rgb(c)
      .map((v) => v / 255)
      .map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4))
      .reduce((sum, v, i) => sum + v * [0.2126, 0.7152, 0.0722][i], 0);
  const x = luminance(a),
    y = luminance(b);
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}
export function readable(color: string, background: string, minimum = 4.5) {
  if (contrast(color, background) >= minimum) return color;
  const target =
    contrast('#ffffff', background) > contrast('#161b19', background) ? '#ffffff' : '#161b19';
  for (let i = 1; i <= 100; i++) {
    const candidate = mix(target, color, i / 100);
    if (contrast(candidate, background) >= minimum) return candidate;
  }
  return target;
}
export function themeTokens(config: AppearanceConfig, dark: boolean) {
  const preset = presets.find((p) => p.id === config.preset) ?? presets[0];
  const accent =
    config.primary && /^#[a-f\d]{6}$/i.test(config.primary) ? config.primary : preset.accent;
  const surface = dark ? mix(preset.tint, '#202323', 0.1) : '#ffffff';
  const base = dark ? mix(preset.tint, '#151819', 0.08) : mix(preset.tint, '#f7f7f3', 0.035);
  const ink = dark ? '#f2f2ec' : '#222a2b';
  const primary = dark ? readable(accent, surface, 3) : accent;
  const primaryInk =
    contrast(primary, '#ffffff') > contrast(primary, '#171b1a') ? '#ffffff' : '#171b1a';
  return {
    '--background': base,
    '--surface': surface,
    '--foreground': ink,
    '--primary': primary,
    '--primary-foreground': primaryInk,
    '--accent-ink': readable(primary, surface),
    '--secondary': mix(preset.tint, surface, dark ? 0.28 : 0.09),
    '--muted': mix(preset.tint, surface, dark ? 0.18 : 0.055),
    '--muted-foreground': readable(dark ? '#a6b0b3' : '#6b7478', surface),
    '--accent': mix(primary, surface, dark ? 0.16 : 0.08),
    '--border': mix(preset.tint, surface, dark ? 0.55 : 0.2),
    '--ring': readable(primary, surface, 3),
    '--subtle': readable(dark ? '#bcc5c6' : '#566367', surface),
    '--yellow': preset.yellow,
    '--green': dark ? '#8ad5b4' : '#227454',
    '--green-soft': dark ? '#223c32' : '#e7f3ed',
    '--danger': dark ? '#ffa39c' : '#ae2632',
    '--red-soft': dark ? '#412b30' : '#fae9ec',
    '--hero': mix(preset.tint, '#1c292d', 0.38),
    '--hero-foreground': '#ffffff',
    '--hero-muted': '#c6d2d4',
    '--chart-1': readable(primary, surface, 3),
    '--chart-2': readable('#2b8a78', surface, 3),
    '--chart-3': readable('#5386ba', surface, 3),
    '--chart-4': readable('#b38632', surface, 3),
    '--chart-5': readable('#8869ac', surface, 3),
    '--chart-6': readable('#c06f4c', surface, 3),
    '--font-scale': String(config.fontScale / 100),
    '--density': String(
      config.density === 'compact' ? 0.82 : config.density === 'comfortable' ? 1.15 : 1,
    ),
    '--shadow': dark ? 'none' : '0 3px 14px #26323a05',
  };
}
interface VisualContext {
  appearance: AppearanceState;
  dark: boolean;
  duration: number;
  motion: boolean;
  tokens: ReturnType<typeof themeTokens>;
  preview: (state: AppearanceState | null) => void;
}
const Context = createContext<VisualContext | null>(null);
export function VisualProvider({
  source,
  children,
}: {
  source: AppearanceState | null | undefined;
  children: ReactNode;
}) {
  const [draft, setDraft] = useState<AppearanceState | null>(null);
  const [systemDark, setSystemDark] = useState(
    () => matchMedia('(prefers-color-scheme:dark)').matches,
  );
  const [systemReduced, setSystemReduced] = useState(
    () => matchMedia('(prefers-reduced-motion:reduce)').matches,
  );
  useEffect(() => {
    const dark = matchMedia('(prefers-color-scheme:dark)'),
      reduced = matchMedia('(prefers-reduced-motion:reduce)');
    const changed = () => {
      setSystemDark(dark.matches);
      setSystemReduced(reduced.matches);
    };
    dark.addEventListener('change', changed);
    reduced.addEventListener('change', changed);
    return () => {
      dark.removeEventListener('change', changed);
      reduced.removeEventListener('change', changed);
    };
  }, []);
  const appearance = draft ?? source ?? defaultAppearance();
  const c = appearance.config;
  const dark = c.mode === 'dark' || (c.mode === 'system' && systemDark);
  const duration =
    systemReduced || c.animations === 'none' ? 0 : c.animations === 'subtle' ? 0.12 : 0.2;
  const tokens = useMemo(() => themeTokens(c, dark), [c, dark]);
  useEffect(() => {
    const root = document.documentElement;
    for (const [key, value] of Object.entries(tokens)) root.style.setProperty(key, value);
    root.style.colorScheme = dark ? 'dark' : 'light';
    root.classList.toggle('dark', dark);
    root.classList.toggle('reduce-motion', duration === 0);
    root.dataset.density = c.density;
    root.dataset.scale = String(c.fontScale);
    root.classList.toggle('custom-background', c.background !== 'theme');
    let background = tokens['--background'];
    if (c.background === 'light') background = '#f4f3ef';
    if (c.background === 'dark') background = '#202728';
    if (c.background === 'solid') background = c.backgroundColor;
    if (c.background === 'gray') background = dark ? '#242729' : '#e8e9e9';
    if (c.background === 'tinted')
      background = mix(tokens['--primary'], tokens['--background'], 0.09);
    root.style.setProperty('--workspace-background', background);
    root.style.setProperty(
      '--workspace-image',
      c.background === 'gradient'
        ? `linear-gradient(135deg,${tokens['--background']},${mix(tokens['--primary'], tokens['--background'], 0.12)})`
        : c.background === 'image' && appearance.backgroundData
          ? `linear-gradient(${tokens['--background']}${Math.round(255 * (1 - c.imageOpacity / 100))
              .toString(16)
              .padStart(2, '0')},${tokens['--background']}${Math.round(
              255 * (1 - c.imageOpacity / 100),
            )
              .toString(16)
              .padStart(
                2,
                '0',
              )}),url("data:image/${appearance.backgroundData.startsWith('iVBOR') ? 'png' : 'jpeg'};base64,${appearance.backgroundData}")`
          : 'none',
    );
    root.style.setProperty('--workspace-position', `${c.imageX}% ${c.imageY}%`);
    root.style.setProperty('--workspace-fit', c.imageFit);
  }, [tokens, dark, duration, c, appearance.backgroundData]);
  const value = useMemo(
    () => ({ appearance, dark, duration, motion: duration > 0, tokens, preview: setDraft }),
    [appearance, dark, duration, tokens],
  );
  return (
    <Context.Provider value={value}>
      <MotionConfig reducedMotion={duration === 0 ? 'always' : 'user'} transition={{ duration }}>
        {children}
      </MotionConfig>
    </Context.Provider>
  );
}
export function useVisual() {
  const value = useContext(Context);
  if (!value) throw new Error('VisualProvider is required');
  return value;
}
