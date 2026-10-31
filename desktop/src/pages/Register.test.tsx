// @vitest-environment jsdom
import { useState } from 'react';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
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
          (!input.brand || p.brand === input.brand) &&
          (!input.search || p.name.toLowerCase().includes(String(input.search).toLowerCase())),
      );
      return {
        items,
        total: items.length,
        page: 0,
        pageSize: 60,
        categories: products.map((p) => ({ name: p.category, count: 1 })),
        brands: products.filter((p) => p.brand).map((p) => ({ name: p.brand, count: 1 })),
      };
    }
    if (op === 'productForCode') {
      const p = products.find((p) => p.barcode === input.code || p.sku === input.code);
      if (!p)
        throw {
          code: 'CODE_NOT_FOUND',
          message: 'Código desconocido',
          incidentId: 'qa-code',
          retryable: false,
        };
      return p;
    }
    if (op === 'cashOverview')
      return { open: { id: 1, folio: 'CAJA-1' }, movements: [], expectedCents: 100000 };
    if (op === 'checkout') {
      const rows = (input.items as Item[]).map((i) => ({
        product: products.find((p) => p.id === i.productId)!,
        ...i,
      }));
      const total = previewAmounts(rows, 0).totalCents;
      return {
        sale: {
          id: 44,
          folio: 'V000044',
          totalCents: total,
          tenderedCents: input.tenderedCents,
          changeCents: Number(input.tenderedCents) - total,
        },
      };
    }
    if (op === 'printDocument')
      throw {
        code: 'PRINTER_UNAVAILABLE',
        message: 'Impresora desconectada',
        incidentId: 'qa-print',
        retryable: false,
      };
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
  await screen.findByRole('button', { name: 'Caja abierta' });
  return screen.getByRole('textbox', { name: 'Capturar código de producto' });
}
async function scan(target: HTMLElement, code: string, suffix = 'Enter') {
  await act(async () => {
    for (const key of code) fireEvent.keyDown(target, { key });
    fireEvent.keyDown(target, { key: suffix });
  });
}
async function picker() {
  fireEvent.keyDown(window, { key: 'F2' });
  const input = await screen.findByRole('combobox', { name: 'Buscar productos' });
  await screen.findByRole('button', { name: 'Agregar Cemento gris' });
  await waitFor(() => expect(document.activeElement).toBe(input));
  return input;
}
const cobrar = () => screen.getByRole('button', { name: /^Cobrar/ }) as HTMLButtonElement;

test('main capture adds exact HID codes directly, repeats with Tab, and handles unknown codes without opening a modal', async () => {
  const capture = await ready();
  await scan(capture, '7500000000001');
  await waitFor(() => expect(current.rows[0]?.quantityUnits).toBe(10000));
  await scan(capture, '7500000000001', 'Tab');
  await waitFor(() => expect(current.rows[0]?.quantityUnits).toBe(20000));
  expect(screen.getByText('Stock bajo · 1 pza')).toBeTruthy();
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(document.activeElement).toBe(capture);
  await scan(capture, '9999999999999');
  await screen.findByText(/Código desconocido/);
  expect(current.rows).toHaveLength(1);
  expect(current.rows[0].quantityUnits).toBe(20000);
  expect(screen.queryByRole('dialog')).toBeNull();
  await waitFor(() => expect(cobrar().disabled).toBe(false));
});

test('F2 filters database categories and brands, rejects exhausted products and preserves the sale on Escape', async () => {
  const capture = await ready();
  await scan(capture, '7500000000001');
  const search = await picker();
  expect(
    (screen.getByRole('button', { name: 'Agregar Brocha agotada' }) as HTMLButtonElement).disabled,
  ).toBe(true);
  expect(screen.getByText('Agotado')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Electricidad' }));
  await waitFor(() =>
    expect(screen.queryByRole('button', { name: 'Agregar Cemento gris' })).toBeNull(),
  );
  expect(screen.getByText('Disponible · 10 m')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Electricidad' }));
  expect(
    (screen.getByRole('button', { name: 'Agregar Cable calibre 12' }) as HTMLButtonElement)
      .disabled,
  ).toBe(false);
  fireEvent.click(screen.getByRole('button', { name: 'Todos' }));
  await screen.findByRole('button', { name: 'Agregar Cemento gris' });
  fireEvent.change(screen.getByLabelText('Filtrar por marca'), { target: { value: 'CEMEX' } });
  await waitFor(() =>
    expect(screen.queryByRole('button', { name: 'Agregar Cable calibre 12' })).toBeNull(),
  );
  expect(current.rows).toHaveLength(1);
  fireEvent.keyDown(search, { key: 'Escape' });
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  await waitFor(() => expect(document.activeElement).toBe(capture));
  expect(current.rows[0].quantityUnits).toBe(10000);
});

test('manual search requires selection; arrows and Enter add then close, fractional edits recalculate and overstock is blocked', async () => {
  const capture = await ready();
  const search = await picker();
  fireEvent.change(search, { target: { value: 'Cable' } });
  await waitFor(() =>
    expect(screen.queryByRole('button', { name: 'Agregar Cemento gris' })).toBeNull(),
  );
  expect(current.rows).toHaveLength(0);
  fireEvent.keyDown(search, { key: 'ArrowDown' });
  fireEvent.keyDown(search, { key: 'Enter' });
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  await waitFor(() => expect(document.activeElement).toBe(capture));
  expect(current.rows).toHaveLength(1);
  const qty = screen.getByRole('textbox', {
    name: 'Cantidad de Cable calibre 12',
  }) as HTMLInputElement;
  fireEvent.change(qty, { target: { value: '2.' } });
  expect(qty.value).toBe('2.');
  fireEvent.change(qty, { target: { value: '2.5' } });
  expect(current.rows[0].quantityUnits).toBe(25000);
  await waitFor(() => expect(screen.getAllByText('$46.25').length).toBeGreaterThan(0));
  fireEvent.change(qty, { target: { value: '11' } });
  expect(current.rows[0].quantityUnits).toBe(25000);
  expect(cobrar().disabled).toBe(true);
  fireEvent.blur(qty);
  expect(qty.value).toBe('2.5');
  fireEvent.click(screen.getByRole('button', { name: 'Quitar Cable calibre 12' }));
  expect(current.rows).toHaveLength(0);
  expect(screen.queryByText(/Agregado:/)).toBeNull();
});

test('scanner inside the picker resolves an exact product even with another category selected, then restores capture focus', async () => {
  const capture = await ready();
  const search = await picker();
  fireEvent.click(screen.getByRole('button', { name: 'Electricidad' }));
  await waitFor(() =>
    expect(screen.queryByRole('button', { name: 'Agregar Cemento gris' })).toBeNull(),
  );
  await scan(search, '7500000000001');
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  await waitFor(() => expect(document.activeElement).toBe(capture));
  expect(current.rows[0].product.sku).toBe('CEM-50');
  expect(current.rows[0].quantityUnits).toBe(10000);
});

test('closing the picker cancels an outstanding exact lookup so a delayed response cannot alter the sale', async () => {
  await ready();
  const search = await picker();
  const original = vi.mocked(api).getMockImplementation()!;
  let resolve!: (value: Product) => void;
  vi.mocked(api).mockImplementation(((op: string, value: unknown) =>
    op === 'productForCode'
      ? new Promise<Product>((done) => {
          resolve = done;
        })
      : original(op as never, value as never)) as typeof api);
  await scan(search, '7500000000001');
  expect(resolve).toBeTypeOf('function');
  fireEvent.keyDown(search, { key: 'Escape' });
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  await act(async () => resolve(products[0]));
  expect(current.rows).toHaveLength(0);
});

test('scanner bursts into quantity and payment fields preserve values and never add or charge', async () => {
  const capture = await ready();
  await scan(capture, '7500000000002');
  const qty = (await screen.findByRole('textbox', {
    name: 'Cantidad de Cable calibre 12',
  })) as HTMLInputElement;
  fireEvent.change(qty, { target: { value: '2.5' } });
  const user = userEvent.setup({ delay: 1 });
  await user.click(qty);
  const before = vi.mocked(api).mock.calls.filter((c) => c[0] === 'productForCode').length;
  await user.type(qty, '7500000000001{Enter}', { skipClick: true });
  expect(current.rows[0].quantityUnits).toBe(25000);
  expect(qty.value).toBe('2.5');
  expect(vi.mocked(api).mock.calls.filter((c) => c[0] === 'productForCode')).toHaveLength(before);
  fireEvent.keyDown(qty, { key: 'Enter' });
  expect(document.activeElement).toBe(capture);
  await waitFor(() => expect(cobrar().disabled).toBe(false));
  fireEvent.keyDown(window, { key: 'F8' });
  const received = (await screen.findByLabelText('Efectivo recibido')) as HTMLInputElement;
  fireEvent.change(received, { target: { value: '100' } });
  await user.click(received);
  await user.type(received, '7500000000001{Enter}', { skipClick: true });
  expect(received.value).toBe('100');
  expect(current.rows).toHaveLength(1);
  expect(vi.mocked(api).mock.calls.some((c) => c[0] === 'checkout')).toBe(false);
  expect(vi.mocked(api).mock.calls.filter((c) => c[0] === 'productForCode')).toHaveLength(before);
});

test('a multi-line cash sale is charged once despite a print failure and capture is ready immediately for the next sale', async () => {
  data.settings = {
    autoPrint: true,
    printerName: 'Impresora de prueba desconectada',
    ticketWidthMm: 80,
  } as Bootstrap['settings'];
  try {
    const capture = await ready();
    await scan(capture, '7500000000001');
    await scan(capture, '7500000000002');
    fireEvent.change(screen.getByRole('textbox', { name: 'Cantidad de Cable calibre 12' }), {
      target: { value: '2.5' },
    });
    await waitFor(() => expect(cobrar().disabled).toBe(false));
    fireEvent.keyDown(window, { key: 'F8' });
    const received = await screen.findByLabelText('Efectivo recibido');
    fireEvent.change(received, { target: { value: '500' } });
    expect(screen.getByText('$218.75')).toBeTruthy();
    const confirm = screen.getByRole('button', { name: 'Confirmar pago' });
    fireEvent.click(confirm);
    fireEvent.click(confirm);
    await screen.findByText('Venta registrada · V000044');
    await waitFor(() => expect(document.activeElement).toBe(capture));
    expect(current.rows).toHaveLength(0);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(vi.mocked(api).mock.calls.filter((c) => c[0] === 'checkout')).toHaveLength(1);
    expect(vi.mocked(api).mock.calls.filter((c) => c[0] === 'printDocument')).toHaveLength(1);
    expect(screen.getByRole('button', { name: 'Ver e imprimir ticket' })).toBeTruthy();
    await scan(capture, '7500000000002');
    await waitFor(() => expect(current.rows).toHaveLength(1));
    expect(screen.queryByText('Venta registrada · V000044')).toBeNull();
  } finally {
    data.settings = undefined as unknown as Bootstrap['settings'];
  }
});

test('emptying a sale requires confirmation and integer units reject fractions', async () => {
  await ready();
  await picker();
  fireEvent.click(screen.getByRole('button', { name: 'Agregar Cemento gris' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  const qty = screen.getByRole('textbox', { name: 'Cantidad de Cemento gris' });
  fireEvent.change(qty, { target: { value: '1.5' } });
  expect(current.rows[0].quantityUnits).toBe(10000);
  expect(cobrar().disabled).toBe(true);
  fireEvent.blur(qty);
  fireEvent.click(screen.getByRole('button', { name: 'Vaciar' }));
  await screen.findByRole('dialog', { name: '¿Vaciar el ticket actual?' });
  expect(current.rows).toHaveLength(1);
  fireEvent.click(screen.getByRole('button', { name: 'Conservar ticket' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(current.rows).toHaveLength(1);
  fireEvent.click(screen.getByRole('button', { name: 'Vaciar' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Vaciar ticket' }));
  expect(current.rows).toHaveLength(0);
});
