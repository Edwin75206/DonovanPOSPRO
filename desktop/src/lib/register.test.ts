import { describe, it, expect } from 'vitest';
import { addQuantity, previewAmounts, ScanBuffer, stockState, validQuantity } from './register';
import type { Product } from './api';
const product = (overrides: Partial<Product> = {}) => ({
  id: 1,
  sku: 'CEM-50',
  barcode: '7500000000001',
  name: 'Cemento',
  category: 'Construcción',
  brand: 'CEMEX',
  size: '50 kg',
  aliases: '',
  unit: 0,
  costCents: null,
  priceCents: 23500,
  wholesaleCents: 22000,
  stockUnits: 50000,
  minimumUnits: 20000,
  taxBasisPoints: 1600,
  packName: 'Caja',
  packUnits: 10000,
  active: true,
  ...overrides,
});
describe('Register rules', () => {
  it('shows remaining stock, warns near minimum, and stops at available quantities', () => {
    const p = product();
    expect(stockState(p, 30000).state).toBe('low');
    expect(stockState(p, 50000).label).toBe('Todo en tu ticket');
    expect(stockState(product({ stockUnits: 0 })).label).toBe('Agotado');
    expect(addQuantity(p, 40000)).toBe(50000);
    expect(addQuantity(p, 50000)).toBeNull();
    expect(validQuantity(p, 15000)).toBe(false);
    expect(validQuantity(p, 60000)).toBe(false);
    const cable = product({ unit: 1, stockUnits: 12500 });
    expect(validQuantity(cable, 2500)).toBe(true);
    expect(addQuantity(cable, 10000)).toBe(12500);
  });
  it('previews fractional prices and discounts in exact cents', () => {
    const rows = [
      { product: product(), quantityUnits: 20000, wholesale: false },
      {
        product: product({ id: 2, unit: 1, priceCents: 1850 }),
        quantityUnits: 25000,
        wholesale: false,
      },
    ];
    expect(previewAmounts(rows, 0).totalCents).toBe(51625);
    const discounted = previewAmounts(rows, 125000);
    expect(discounted.totalCents).toBe(45172);
    expect(discounted.lineCents.reduce((a, b) => a + b, 0)).toBe(45172);
  });
  it('accepts consecutive scans with either suffix, but ignores manual typing and editing fields', () => {
    const buffer = new ScanBuffer();
    let time = 1000;
    const scan = (code: string, suffix = 'Enter', allowed = true) => {
      for (const key of code) buffer.key(key, (time += 8), allowed);
      return buffer.key(suffix, (time += 8), allowed);
    };
    expect(scan('7500000000001')).toBe('7500000000001');
    expect(scan('7500000000001', 'Tab')).toBe('7500000000001');
    for (const key of 'cemento') buffer.key(key, (time += 110), true);
    expect(buffer.key('Enter', (time += 100), true)).toBeNull();
    expect(scan('7500000000001', 'Enter', false)).toBeNull();
    scan('CEM-50', 'Escape');
    expect(buffer.key('Enter', time + 10, true)).toBeNull();
  });
});
