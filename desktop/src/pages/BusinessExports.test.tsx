// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { api, type Bootstrap } from '@/lib/api';
import { Settings } from './Settings';

vi.mock('@/lib/api', async (original) => ({
  ...(await original<typeof import('@/lib/api')>()),
  api: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

const settings = () =>
  render(
    <Settings
      data={{} as Bootstrap}
      onChanged={() => {}}
      onSession={() => {}}
      hasDraft={false}
      filters={{ tab: 'exports' }}
    />,
  );

test('routes each Excel export from business settings to the selected dataset', async () => {
  vi.mocked(api).mockResolvedValue({ path: 'C:/Archivos/Donovan.xlsx', bytes: 4096 });
  settings();
  expect(
    screen.getByRole('button', { name: 'Exportar a Excel' }).getAttribute('aria-current'),
  ).toBe('page');
  const options = [
    ['Exportar ventas de hoy', 'salesToday'],
    ['Exportar todas las ventas', 'sales'],
    ['Exportar clientes', 'customers'],
    ['Exportar proveedores', 'suppliers'],
    ['Exportar productos e inventario', 'inventory'],
    ['Exportar cotizaciones', 'quotes'],
    ['Exportar compras', 'purchases'],
    ['Exportar devoluciones', 'returns'],
    ['Exportar caja', 'cash'],
    ['Exportar crédito', 'credit'],
    ['Exportar toda la base a Excel', 'all'],
  ];
  for (const [label, scope] of options) {
    await act(async () => fireEvent.click(screen.getByRole('button', { name: label })));
    expect(api).toHaveBeenLastCalledWith('exportBusiness', { scope });
    expect(screen.getByText('C:/Archivos/Donovan.xlsx')).toBeTruthy();
  }
  expect(api).toHaveBeenCalledTimes(options.length);
});

test('cancelling the folder picker re-enables exports without reporting a saved file', async () => {
  let finish!: (value: null) => void;
  vi.mocked(api).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  settings();
  const button = screen.getByRole('button', { name: 'Exportar toda la base a Excel' });
  fireEvent.click(button);
  expect(button.hasAttribute('disabled')).toBe(true);
  expect(screen.getByText('Preparando el archivo de Excel…')).toBeTruthy();
  await act(async () => finish(null));
  await waitFor(() => expect(button.hasAttribute('disabled')).toBe(false));
  expect(screen.queryByText('Archivo de Excel guardado')).toBeNull();
});
