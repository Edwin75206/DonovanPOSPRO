// @vitest-environment jsdom
import { useState } from 'react';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { api, type Product, type Bootstrap, type Item } from '@/lib/api';
import { previewAmounts } from '@/lib/register';
import { Register, freshCart, type Cart } from './Register';
vi.mock('@/lib/api', async (original) => ({
  ...(await original<typeof import('@/lib/api')>()),
  api: vi.fn(),
}));
vi.mock('@/lib/appearance', () => ({ useVisual: () => ({ duration: 0 }) }));
const products: Product[] = [
  {
    id: 1,
    sku: 'CEM-50',
    barcode: '7500000000001',
    name: 'Cemento gris',
    category: 'Construcción',
    brand: 'CEMEX',
    size: '50 kg',
    unit: 0,
    stockUnits: 30000,
    minimumUnits: 10000,
    priceCents: 23500,
  },
  {
    id: 2,
    sku: 'CAB-12',
    barcode: '7500000000002',
    name: 'Cable calibre 12',
    category: 'Electricidad',
    brand: 'Condumex',
    unit: 1,
    stockUnits: 100000,
    minimumUnits: 50000,
    priceCents: 1850,
  },
  {
    id: 3,
    sku: 'BRO-2',
    barcode: '7500000000003',
    name: 'Brocha agotada',
    category: 'Pintura',
    unit: 0,
    stockUnits: 0,
    minimumUnits: 20000,
    priceCents: 4800,
  },
].map((p) => ({
  aliases: '',
  brand: '',
  size: '',
  wholesaleCents: 0,
  costCents: null,
  taxBasisPoints: 1600,
  packName: '',
  packUnits: 10000,
  active: true,
  ...p,
}));
const data = {
  actor: { id: 2, username: 'vendedor', name: 'Negociante Uno', role: 1 },
} as Bootstrap;
let current: Cart;
function Harness() {
  const [cart, setCart] = useState(freshCart);
  current = cart;
  return (
    <Register
      data={data}
      cart={cart}
      setCart={setCart}
      onCash={() => {}}
      onSaved={() => {}}
      onHistory={() => {}}
      onQuotes={() => {}}
    />
  );
}
beforeEach(() => {
  vi.mocked(api).mockReset();
  vi.mocked(api).mockImplementation((async (op: string, input: Record<string, unknown> = {}) => {
    if (op === 'products') {
      const items = products.filter(
        (p) =>
          (!input.category || p.category === input.category) &&
          (!input.search || p.name.toLowerCase().includes(String(input.search).toLowerCase())),
      );
      return {
        items,
        total: items.length,
        page: 0,
        pageSize: 60,
        categories: products.map((p) => ({ name: p.category, count: 1 })),
      };
    }
    if (op === 'productForCode') {
      const p = products.find((p) => p.barcode === input.code || p.sku === input.code);
      if (!p) throw Error('Código desconocido');
      return p;
    }
    if (op === 'cashOverview')
      return { open: { id: 1, folio: 'CAJA-1' }, movements: [], expectedCents: 100000 };
    if (op === 'price') {
      const rows = (input.items as Item[]).map((i) => ({
        product: products.find((p) => p.id === i.productId)!,
        ...i,
      }));
      const sums = previewAmounts(rows, 0);
      return {
        ...sums,
        taxCents: 0,
        lines: rows.map((r, i) => ({
          ...r.product,
          productId: r.product.id,
          quantityUnits: r.quantityUnits,
          totalCents: sums.lineCents[i],
          taxCents: 0,
        })),
      };
    }
    throw Error(`Unexpected ${op}`);
  }) as typeof api);
});
afterEach(() => {
  cleanup();
});
async function ready() {
  render(<Harness />);
  await screen.findByRole('button', { name: /Cemento gris.*Agregar/ });
  return screen.getByRole('textbox', { name: 'Buscar o escanear producto' });
}
async function scan(target: HTMLElement, code: string, suffix = 'Enter') {
  await act(async () => {
    for (const key of code) fireEvent.keyDown(target, { key });
    fireEvent.keyDown(target, { key: suffix });
  });
}
test('HID scans add once, repeat, survive categories and ignore quantity/payment inputs', async () => {
  const search = await ready();
  await scan(search, '7500000000001');
  await waitFor(() => expect(current.rows[0]?.quantityUnits).toBe(10000));
  await scan(search, '7500000000001', 'Tab');
  await waitFor(() => expect(current.rows[0]?.quantityUnits).toBe(20000));
  expect(screen.getByText('Últimas 1 pza')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Electricidad' }));
  await waitFor(() =>
    expect(screen.queryByRole('button', { name: /Cemento gris.*Agregar/ })).toBeNull(),
  );
  expect(current.rows[0].quantityUnits).toBe(20000);
  const qty = screen.getByRole('textbox', { name: 'Cantidad de Cemento gris' });
  const before = vi.mocked(api).mock.calls.filter((c) => c[0] === 'productForCode').length;
  await scan(qty, '7500000000002');
  expect(vi.mocked(api).mock.calls.filter((c) => c[0] === 'productForCode')).toHaveLength(before);
  expect(current.rows).toHaveLength(1);
  await waitFor(() =>
    expect(
      (screen.getByRole('button', { name: /Cobrar venta/ }) as HTMLButtonElement).disabled,
    ).toBe(false),
  );
  fireEvent.keyDown(window, { key: 'F8' });
  await screen.findByRole('dialog', { name: 'Cobrar y entregar.' });
  await scan(screen.getByLabelText('Efectivo recibido'), '7500000000002');
  expect(current.rows).toHaveLength(1);
  expect(vi.mocked(api).mock.calls.some((c) => c[0] === 'checkout')).toBe(false);
});
test('manual search needs a deliberate selection; fractional quantity updates total and rejects overstock', async () => {
  const search = await ready();
  expect(
    (screen.getByRole('button', { name: /Brocha agotada.*Agregar/ }) as HTMLButtonElement).disabled,
  ).toBe(true);
  fireEvent.change(search, { target: { value: 'Cable' } });
  await waitFor(() =>
    expect(screen.queryByRole('button', { name: /Cemento gris.*Agregar/ })).toBeNull(),
  );
  fireEvent.keyDown(search, { key: 'Enter' });
  expect(current.rows).toHaveLength(0);
  fireEvent.keyDown(search, { key: 'ArrowDown' });
  fireEvent.keyDown(search, { key: 'Enter' });
  await waitFor(() => expect(current.rows).toHaveLength(1));
  const qty = screen.getByRole('textbox', { name: 'Cantidad de Cable calibre 12' });
  fireEvent.change(qty, { target: { value: '2.' } });
  expect((qty as HTMLInputElement).value).toBe('2.');
  fireEvent.change(qty, { target: { value: '2.5' } });
  expect(current.rows[0].quantityUnits).toBe(25000);
  await waitFor(() => expect(screen.getAllByText('$46.25').length).toBeGreaterThan(0));
  fireEvent.change(qty, { target: { value: '11' } });
  expect(current.rows[0].quantityUnits).toBe(25000);
  expect((screen.getByRole('button', { name: /Revisa/ }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.blur(qty);
  expect((qty as HTMLInputElement).value).toBe('2.5');
  fireEvent.click(screen.getByRole('button', { name: 'Quitar Cable calibre 12' }));
  await waitFor(() => expect(current.rows).toHaveLength(0));
  expect(screen.queryByText(/en tu ticket/)).toBeNull();
});
