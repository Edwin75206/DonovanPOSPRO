// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { api, type Bootstrap, type Payment, type SaleDetail } from '@/lib/api';
import { Sales } from './Sales';

vi.mock('@/lib/api', async (original) => ({
  ...(await original<typeof import('@/lib/api')>()),
  api: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

async function openCancellation(payments: Payment[]) {
  let detail: SaleDetail = {
    sale: {
      id: 2,
      folio: 'V-000002',
      requestId: 'checkout-2',
      createdAt: '2026-11-24 12:00:00',
      userId: 1,
      customerId: null,
      customerName: 'Público en general',
      cashier: 'Administrador',
      totalCents: 10000,
      taxCents: 0,
      discountCents: 0,
      changeCents: 0,
      tenderedCents: payments.find((p) => p.method === 0)?.amountCents ?? 0,
      cancelled: false,
    },
    lines: [
      {
        id: 10,
        productId: 5,
        sku: 'PZA',
        name: 'Pieza',
        category: 'Material',
        unit: 0,
        quantityUnits: 10000,
        priceCents: 10000,
        costCents: null,
        taxBasisPoints: 0,
        totalCents: 10000,
        taxCents: 0,
        returnedUnits: 0,
        refundedCents: 0,
      },
    ],
    payments,
    returns: [],
    creditBalanceCents: 0,
    canDelete: false,
  };
  vi.mocked(api).mockImplementation(((op: string) => {
    if (op === 'sale') return Promise.resolve(detail);
    if (op === 'salesPage')
      return Promise.resolve({
        items: [{ ...detail.sale, refundedCents: detail.sale.cancelled ? 10000 : 0 }],
        total: 1,
        totalCents: 10000,
        refundedCents: detail.sale.cancelled ? 10000 : 0,
      });
    if (op === 'returnSale') {
      detail = { ...detail, sale: { ...detail.sale, cancelled: true } };
      return Promise.resolve({
        return: { folio: 'DV-000001', creditReductionCents: 0 },
        paidRefundCents: 10000,
        cancelled: true,
      });
    }
    throw new Error(`Unexpected API call: ${op}`);
  }) as typeof api);
  render(<Sales data={{ actor: { id: 1, role: 0 } } as Bootstrap} filters={{ saleId: 2 }} />);
  const detailDialog = await screen.findByRole('dialog', { name: 'Venta V-000002' });
  expect(within(detailDialog).getByRole('button', { name: 'Devolver productos' })).toBeTruthy();
  fireEvent.click(within(detailDialog).getByRole('button', { name: 'Cancelar venta' }));
  const cancel = await screen.findByRole('dialog', { name: 'Cancelar venta · V-000002' });
  fireEvent.change(within(cancel).getByRole('textbox', { name: 'Motivo' }), {
    target: { value: 'Pedido cancelado' },
  });
  return within(cancel);
}

test('cancels the whole remaining sale, previews the cash withdrawal and keeps the cancelled record visible', async () => {
  const cancel = await openCancellation([{ method: 0, amountCents: 10000, reference: '' }]);
  expect(
    (cancel.getByRole('combobox', { name: 'Forma de reembolso' }) as HTMLSelectElement).value,
  ).toBe('0');
  expect(cancel.getByText('Se descontarán $100.00 del efectivo de la caja abierta.')).toBeTruthy();
  fireEvent.click(cancel.getByRole('checkbox'));
  fireEvent.click(cancel.getByRole('button', { name: 'Confirmar cancelación' }));
  const result = await screen.findByRole('dialog', { name: 'Venta V-000002' });
  expect(await within(result).findByText('Cancelada')).toBeTruthy();
  expect(within(result).queryByRole('button', { name: 'Cancelar venta' })).toBeNull();
  expect(api).toHaveBeenCalledWith(
    'returnSale',
    expect.objectContaining({
      saleId: 2,
      cancel: true,
      method: 0,
      expectedTotalCents: 10000,
      expectedPaidRefundCents: 10000,
    }),
  );
  fireEvent.click(within(result).getByRole('button', { name: 'Cerrar' }));
  const filter = screen.getByRole('combobox', { name: 'Estado de la venta' });
  fireEvent.change(filter, { target: { value: 'cancelled' } });
  expect(await screen.findByText('Cancelada')).toBeTruthy();
  await waitFor(() =>
    expect(api).toHaveBeenCalledWith('salesPage', expect.objectContaining({ status: 'cancelled' })),
  );
});

test('requires a refund method for mixed payments before confirming a cancellation', async () => {
  const cancel = await openCancellation([
    { method: 0, amountCents: 4000, reference: '' },
    { method: 1, amountCents: 6000, reference: '' },
  ]);
  const method = cancel.getByRole('combobox', { name: 'Forma de reembolso' }) as HTMLSelectElement;
  expect(method.value).toBe('');
  fireEvent.click(cancel.getByRole('checkbox'));
  expect(
    cancel.getByRole('button', { name: 'Confirmar cancelación' }).hasAttribute('disabled'),
  ).toBe(true);
  fireEvent.change(method, { target: { value: '1' } });
  expect(cancel.queryByText(/efectivo de la caja abierta/)).toBeNull();
  expect(
    cancel.getByText('Registra esta operación después de devolver $100.00 por tarjeta.'),
  ).toBeTruthy();
});
