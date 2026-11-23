import { useState, type RefObject } from 'react';
import {
  BrickWall,
  Cable,
  Construction,
  Droplets,
  FlaskConical,
  Hammer,
  HardHat,
  Layers,
  Nut,
  Package,
  PaintRoller,
  Ruler,
  ShoppingBag,
  Sparkles,
  Zap,
} from 'lucide-react';
import { Button, Modal } from '@/components/common';
import type { Product, ProductAppearance } from '@/lib/api';

const icons = [
  { key: 'cemento', label: 'Cemento y mezclas', Icon: ShoppingBag, pattern: /cemento|mortero|yeso/ },
  { key: 'bloque', label: 'Block y ladrillo', Icon: BrickWall, pattern: /block|bloque|ladrillo/ },
  { key: 'acero', label: 'Acero y varilla', Icon: Construction, pattern: /acero|varilla|alambron/ },
  { key: 'madera', label: 'Madera', Icon: Layers, pattern: /madera|tablon|triplay/ },
  { key: 'pintura', label: 'Pintura', Icon: PaintRoller, pattern: /pintura|brocha|rodillo/ },
  { key: 'cable', label: 'Cable', Icon: Cable, pattern: /cable/ },
  { key: 'electricidad', label: 'Electricidad', Icon: Zap, pattern: /electric|foco|contacto/ },
  { key: 'plomeria', label: 'Plomería', Icon: Droplets, pattern: /plomeria|tubo|valvula|llave de agua/ },
  { key: 'proteccion', label: 'Protección', Icon: HardHat, pattern: /seguridad|casco|guante/ },
  { key: 'tornilleria', label: 'Tornillería', Icon: Nut, pattern: /tornillo|tuerca|tornilleria/ },
  { key: 'medicion', label: 'Medición', Icon: Ruler, pattern: /cinta metrica|flexometro|medicion/ },
  { key: 'adhesivos', label: 'Adhesivos y selladores', Icon: FlaskConical, pattern: /adhesivo|sellador|pegamento/ },
  { key: 'herramientas', label: 'Herramientas', Icon: Hammer, pattern: /herramienta|martillo|pinza|taladro/ },
  { key: 'paquete', label: 'Paquete', Icon: Package },
];

const colors = [
  { key: 'amarillo', label: 'Amarillo' },
  { key: 'naranja', label: 'Naranja' },
  { key: 'rojo', label: 'Rojo' },
  { key: 'verde', label: 'Verde' },
  { key: 'azul', label: 'Azul' },
  { key: 'morado', label: 'Morado' },
  { key: 'agua', label: 'Turquesa' },
  { key: 'gris', label: 'Gris' },
];

type ProductIdentity = Pick<Product, 'name' | 'category' | 'appearance'>;

export function ProductVisual({ product }: { product: ProductIdentity }) {
  const material = (product.name + ' ' + product.category)
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase();
  const visual =
    icons.find(({ key }) => key === product.appearance?.icon) ??
    icons.find(({ pattern }) => pattern?.test(material));
  const Icon = visual?.Icon ?? Package;
  const color = colors.find(({ key }) => key === product.appearance?.color)?.key ?? 'gris';
  return (
    <span className="material-icon" data-material-color={color} aria-hidden="true">
      <Icon size={21} strokeWidth={1.8} />
    </span>
  );
}

export function ProductAppearancePicker({
  product,
  onClose,
  onApply,
  returnFocusRef,
}: {
  product: ProductIdentity;
  onClose: () => void;
  onApply: (appearance: ProductAppearance) => void;
  returnFocusRef: RefObject<HTMLButtonElement | null>;
}) {
  const [draft, setDraft] = useState<ProductAppearance>(
    product.appearance ?? { icon: '', color: '' },
  );
  return (
    <Modal
      open
      title="Icono y color"
      className="material-picker-dialog"
      onClose={onClose}
      returnFocusRef={returnFocusRef}
    >
      <div className="material-picker-preview">
        <ProductVisual product={{ ...product, appearance: draft }} />
        <strong>{product.name.trim() || 'Producto nuevo'}</strong>
      </div>
      <fieldset className="material-picker-group">
        <legend>Icono de material</legend>
        <div className="material-picker-options">
          {[{ key: '', label: 'Automático', Icon: Sparkles }, ...icons].map(({ key, label, Icon }) => (
            <button
              key={key}
              type="button"
              aria-pressed={draft.icon === key}
              onClick={() => setDraft({ ...draft, icon: key })}
            >
              <Icon size={21} aria-hidden="true" />
              {label}
            </button>
          ))}
        </div>
      </fieldset>
      <fieldset className="material-picker-group">
        <legend>Color</legend>
        <div className="material-picker-options material-color-options">
          {[{ key: '', label: 'Predeterminado' }, ...colors].map(({ key, label }) => (
            <button
              key={key}
              type="button"
              aria-pressed={draft.color === key}
              onClick={() => setDraft({ ...draft, color: key })}
            >
              <span data-material-color={key || 'gris'} className="material-swatch" aria-hidden="true" />
              {label}
            </button>
          ))}
        </div>
      </fieldset>
      <div className="dialog-actions">
        <Button type="button" variant="outline" onClick={onClose}>Cancelar</Button>
        <Button type="button" onClick={() => onApply(draft)}>Aplicar</Button>
      </div>
    </Modal>
  );
}
