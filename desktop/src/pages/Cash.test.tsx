// @vitest-environment jsdom
import { useState } from 'react';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { api, type Bootstrap, type CashOverview, type SaleDetail } from '@/lib/api';
import { Cash } from './Cash';
import { Sales } from './Sales';

vi.mock('@/lib/api', async (original) => ({
  ...(await original<typeof import('@/lib/api')>()),
  api: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

test('a cash sale opens the matching record in the sales detail dialog', async () => {
  const createdAt = '2026-11-21 17:34:00';
  const cash: CashOverview = {
    open: null,
    expectedCents: 127200,
    movementCount: 2,
    movements: [
      { id: 4, saleId: 17, createdAt, amountCents: 127200, kind: 'Venta', reference: 'V-000017', reason: '' },
      { id: 5, saleId: null, createdAt, amountCents: 10000, kind: 'Ingreso', reference: 'CJ-000001', reason: '' },
    ],
    history: [],
  };
  const detail: SaleDetail = {
    sale: {
      id: 17,
      folio: 'V-000017',
      requestId: 'checkout-17',
      createdAt,
      userId: 2,
      customerId: null,
      customerName: 'Público en general',
      cashier: 'Caja',
      totalCents: 127200,
      taxCents: 17545,
      discountCents: 0,
      changeCents: 72800,
      tenderedCents: 200000,
      cancelled: false,
    },
    lines: [],
    payments: [{ method: 0, amountCents: 127200, reference: '' }],
    returns: [],
    creditBalanceCents: 0,
  };
  const salesPage = { items: [], total: 0, page: 0, pageSize: 30 };
  vi.mocked(api).mockImplementation(((method: string) => {
    if (method === 'cashOverview') return Promise.resolve(cash);
    if (method === 'sale') return Promise.resolve(detail);
    return Promise.resolve(salesPage);
  }) as typeof api);

  function Workspace() {
    const [saleId, setSaleId] = useState<number | null>(null);
    return saleId === null
      ? <Cash onViewSale={setSaleId} />
      : <Sales data={{} as Bootstrap} filters={{ saleId }} />;
  }

  render(<Workspace />);
  const folio = await screen.findByRole('button', { name: 'V-000017' });
  expect(screen.queryByRole('button', { name: 'CJ-000001' })).toBeNull();
  fireEvent.click(folio);
  const dialog = await screen.findByRole('dialog', { name: 'Venta V-000017' });
  expect(within(dialog).getByRole('button', { name: 'Ver y reimprimir ticket' })).toBeTruthy();
  expect(vi.mocked(api)).toHaveBeenCalledWith('sale', { id: 17 });
});
