// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DataGrid } from './DataGrid';
describe('Operational tables', () => {
  it('sorts numeric amounts and pages without discarding records', async () => {
    const user = userEvent.setup();
    const rows = Array.from({ length: 31 }, (_, i) => ({
      id: i + 1,
      name: `Producto ${i + 1}`,
      price: 31 - i,
    }));
    render(
      <DataGrid
        caption="Catálogo de prueba"
        rows={rows}
        columns={[
          { id: 'name', label: 'Producto', value: (r) => r.name },
          { id: 'price', label: 'Precio', value: (r) => r.price },
        ]}
      />,
    );
    expect(screen.getByText('Producto 1')).toBeTruthy();
    expect(screen.queryByText('Producto 31')).toBeNull();
    await user.click(screen.getByRole('button', { name: 'Siguiente' }));
    expect(screen.getByText('Producto 31')).toBeTruthy();
    await user.click(screen.getByRole('button', { name: 'Anterior' }));
    await user.click(screen.getByRole('button', { name: 'Precio' }));
    const body = screen.getByRole('table').querySelector('tbody')!;
    const first = body.querySelector('tr')!;
    expect(first.textContent).toContain('Producto 1');
    await user.click(screen.getByRole('button', { name: 'Precio' }));
    expect(body.querySelector('tr')!.textContent).toContain('Producto 31');
  });
});
