// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { api, type Actor, type Bootstrap } from '@/lib/api';
import { Inventory, newProduct } from './Inventory';

vi.mock('@/lib/api', async (original) => ({
  ...(await original<typeof import('@/lib/api')>()),
  api: vi.fn(),
}));

const actor: Actor = { id: 1, role: 0, username: 'admin', name: 'Administración' };
const cashier: Actor = { id: 2, role: 1, username: 'wendy', name: 'Wendy Calderon' };
const filters = {};
const item = { ...newProduct(), id: 7, sku: 'CEM-50', name: 'Cemento gris', stockUnits: 50000 };

beforeEach(() => {
  let current = item;
  vi.mocked(api)
    .mockReset()
    .mockImplementation((async (name: string, payload: unknown) => {
      if (name === 'inventory') {
        return {
          items: [current], total: 1, page: 0, pageSize: 60,
          categories: [{ name: 'Construcción', count: 1 }], brands: [],
        };
      }
      if (name === 'adjustStock') {
        current = { ...current, stockUnits: current.stockUnits + (payload as { delta: number }).delta };
        return current;
      }
      return payload;
    }) as typeof api);
});
afterEach(cleanup);

async function openInventory(user = actor) {
  render(<Inventory actor={user} business={{} as Bootstrap} filters={filters} />);
  return screen.findByRole('button', { name: /Cemento gris.*CEM-50/ }, { timeout: 10000 });
}

test('creates a category above the product form and saves the amount typed in pesos and initial stock', async () => {
  await openInventory();
  const user = userEvent.setup();
  await user.click(screen.getByRole('button', { name: 'Nuevo producto' }));
  const product = screen.getByRole('dialog', { name: 'Agrega un producto' });
  const fields = within(product);
  fireEvent.change(fields.getByLabelText('Descripción del producto'), {
    target: { value: 'Casco' },
  });
  fireEvent.change(fields.getByLabelText('Clave interna'), { target: { value: 'CASCO' } });
  const categories = fields.getByLabelText<HTMLSelectElement>('Categoría');
  expect(Array.from(categories.options, (option) => option.value)).toContain('Construcción');
  await user.selectOptions(categories, '');
  const category = screen.getByRole('dialog', { name: 'Agregar categoría' });
  const name = within(category).getByLabelText('Nombre de la categoría');
  await waitFor(() => expect(document.activeElement).toBe(name));
  await user.type(name, 'Seguridad');
  await user.click(within(category).getByRole('button', { name: 'Agregar y seleccionar' }));
  await waitFor(() => expect(document.activeElement).toBe(categories));
  expect(categories.value).toBe('Seguridad');
  expect(fields.getByLabelText<HTMLInputElement>('Descripción del producto').value).toBe('Casco');
  expect(vi.mocked(api).mock.calls.some(([method]) => method === 'saveProduct')).toBe(false);

  const price = fields.getByLabelText<HTMLInputElement>('Precio público (IVA incluido)');
  await user.type(price, '12345.67');
  expect(price.value).toBe('12,345.67');
  expect(price.selectionStart).toBe(price.value.length);
  fireEvent.change(fields.getByLabelText('Existencia inicial (pza)'), { target: { value: '8' } });
  await user.click(fields.getByRole('button', { name: 'Guardar producto' }));
  expect(vi.mocked(api).mock.calls).toContainEqual([
    'saveProduct',
    expect.objectContaining({ category: 'Seguridad', priceCents: 1234567, stockUnits: 80000 }),
  ]);
});

test('keeps three row actions for editing, adjusting and history, with labels inside the editor', async () => {
  const rowButton = await openInventory();
  const actions = within(rowButton.closest('tr')!.lastElementChild as HTMLElement);
  expect(actions.getAllByRole('button').map((button) => button.textContent)).toEqual([
    'Editar', 'Ajustar existencias', 'Historial',
  ]);
  fireEvent.click(actions.getByRole('button', { name: 'Editar Cemento gris' }));
  const editor = within(screen.getByRole('dialog', { name: 'Ficha del producto' }));
  expect(editor.getByRole('button', { name: 'Imprimir etiquetas' })).toBeTruthy();
  expect(editor.getByLabelText('Precio público (IVA incluido)').getAttribute('aria-describedby')).toBeTruthy();
  expect(editor.getByText(/A esta cantidad o menos aparece Stock bajo/)).toBeTruthy();
});

test('a cashier can add and remove quantities from the product without changing its details', async () => {
  const rowButton = await openInventory(cashier);
  const user = userEvent.setup();
  const actions = within(rowButton.closest('tr')!.lastElementChild as HTMLElement);
  expect(actions.getAllByRole('button').map((button) => button.textContent)).toEqual([
    'Ver ficha', 'Ajustar existencias', 'Historial',
  ]);
  await user.click(actions.getByRole('button', { name: 'Ver ficha de Cemento gris' }));
  const product = screen.getByRole('dialog', { name: 'Consulta de producto' });
  const view = within(product);
  expect(view.queryByRole('button', { name: 'Guardar producto' })).toBeNull();
  expect(view.queryByLabelText('Costo base sin IVA')).toBeNull();
  const trigger = view.getByRole('button', { name: 'Ajustar existencias' });
  await user.click(trigger);
  let adjust = within(screen.getByRole('dialog', { name: 'Ajustar existencias' }));
  await user.type(adjust.getByLabelText('Cantidad a agregar (pza)'), '50');
  expect(adjust.getByText('55 pza')).toBeTruthy();
  await user.click(adjust.getByRole('button', { name: 'Guardar ajuste' }));
  await waitFor(() => {
    expect(view.getByLabelText<HTMLInputElement>('Existencia actual (pza)').value).toBe('55');
    expect(document.activeElement).toBe(trigger);
  });
  await user.click(trigger);
  adjust = within(screen.getByRole('dialog', { name: 'Ajustar existencias' }));
  await user.click(adjust.getByRole('button', { name: 'Quitar' }));
  const amount = adjust.getByLabelText('Cantidad a quitar (pza)');
  fireEvent.change(amount, { target: { value: '56' } });
  const save = adjust.getByRole<HTMLButtonElement>('button', { name: 'Guardar ajuste' });
  expect(save.disabled).toBe(true);
  expect(adjust.getByRole('alert').textContent).toContain('supera las existencias');
  fireEvent.change(amount, { target: { value: '1' } });
  await user.click(save);
  await waitFor(() => expect(view.getByLabelText<HTMLInputElement>('Existencia actual (pza)').value).toBe('54'));
  expect(vi.mocked(api).mock.calls.filter(([method]) => method === 'adjustStock')).toEqual([
    ['adjustStock', { id: 7, delta: 500000, reason: '' }],
    ['adjustStock', { id: 7, delta: -10000, reason: '' }],
  ]);
});

test('edits the current stock and accepts an empty adjustment reason', async () => {
  const row = await openInventory();
  const user = userEvent.setup();
  await user.click(row);
  const fields = within(screen.getByRole('dialog', { name: 'Ficha del producto' }));
  const stock = fields.getByLabelText<HTMLInputElement>('Existencia actual (pza)');
  expect(stock.value).toBe('5');
  fireEvent.change(stock, { target: { value: '7' } });
  expect(fields.getByLabelText<HTMLInputElement>('Motivo del ajuste (opcional)').required).toBe(
    false,
  );
  await user.click(fields.getByRole('button', { name: 'Guardar producto' }));
  expect(vi.mocked(api).mock.calls).toContainEqual([
    'saveProduct',
    expect.objectContaining({
      stockUnits: 70000,
      inventoryAdjustment: { expectedStockUnits: 50000, reason: '' },
    }),
  ]);
});

test('chooses a local material icon and color while keeping the product form open', async () => {
  const row = await openInventory();
  const user = userEvent.setup();
  await user.click(row);
  const product = screen.getByRole('dialog', { name: 'Ficha del producto' });
  const editor = within(product);
  await user.click(editor.getByRole('button', { name: 'Elegir icono y color' }));
  const chooser = within(screen.getByRole('dialog', { name: 'Icono y color' }));
  await user.click(chooser.getByRole('button', { name: 'Cemento y mezclas' }));
  await user.click(chooser.getByRole('button', { name: 'Azul' }));
  expect(chooser.getByRole('button', { name: 'Azul', pressed: true })).toBeTruthy();
  await user.click(chooser.getByRole('button', { name: 'Aplicar' }));
  await waitFor(() => {
    expect(document.activeElement).toBe(editor.getByRole('button', { name: 'Elegir icono y color' }));
  });
  expect(editor.getByLabelText<HTMLInputElement>('Descripción del producto').value).toBe('Cemento gris');
  await user.click(editor.getByRole('button', { name: 'Guardar producto' }));
  expect(vi.mocked(api).mock.calls).toContainEqual([
    'saveProduct',
    expect.objectContaining({ appearance: { icon: 'cemento', color: 'azul' } }),
  ]);
});
