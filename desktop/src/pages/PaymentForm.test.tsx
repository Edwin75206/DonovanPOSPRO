// @vitest-environment jsdom
import { afterEach, beforeEach, describe, it, expect, vi } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { PaymentForm } from './PaymentForm';
import { freshCart } from './Register';
import { api, type Charge } from '@/lib/api';
vi.mock('@/lib/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/api')>()),
  api: vi.fn(),
}));
const charge = {
  totalCents: 17050,
  taxCents: 2352,
  discountCents: 0,
  lines: [{ productId: 1, name: 'Producto de prueba', quantityUnits: 10000, totalCents: 17050 }],
} as Charge;
beforeEach(() => {
  vi.mocked(api).mockReset();
});
afterEach(cleanup);
const setup = () => {
  const onSuccess = vi.fn();
  render(
    <PaymentForm
      charge={charge}
      cart={freshCart()}
      allowCredit={false}
      onBusy={() => {}}
      onSuccess={onSuccess}
    />,
  );
  return { user: userEvent.setup(), onSuccess };
};
describe('Cash at the register', () => {
  it('rejects incomplete cash, calculates change and submits one sale on double click', async () => {
    const { user, onSuccess } = setup();
    let release: (value: unknown) => void = () => {};
    vi.mocked(api).mockImplementation(
      () =>
        new Promise((resolve) => {
          release = resolve;
        }) as never,
    );
    fireEvent.change(screen.getByLabelText('Efectivo recibido'), { target: { value: '100' } });
    expect(
      (screen.getByRole('button', { name: /Confirmar pago/ }) as HTMLButtonElement).disabled,
    ).toBe(true);
    await user.click(screen.getByRole('button', { name: '$200.00' }));
    expect(screen.getByText('$29.50')).toBeTruthy();
    await user.dblClick(screen.getByRole('button', { name: /Confirmar pago/ }));
    expect(api).toHaveBeenCalledTimes(1);
    expect(vi.mocked(api).mock.calls[0][1]).toMatchObject({
      tenderedCents: 20000,
      payments: [{ method: 0, amountCents: 17050 }],
    });
    release({ sale: { id: 1 } });
    await waitFor(() => expect(onSuccess).toHaveBeenCalledOnce());
  });
  it('never gives change on a manual card payment', async () => {
    const { user } = setup();
    vi.mocked(api).mockResolvedValue({ sale: { id: 2 } } as never);
    await user.click(screen.getByRole('button', { name: 'Tarjeta' }));
    expect(screen.queryByText('Cambio para el cliente')).toBeNull();
    await user.click(screen.getByRole('button', { name: /Confirmar pago/ }));
    expect(vi.mocked(api).mock.calls[0][1]).toMatchObject({
      tenderedCents: 0,
      payments: [{ method: 1, amountCents: 17050 }],
    });
  });
  it('calculates change only on the cash portion of mixed payments', async () => {
    const { user } = setup();
    vi.mocked(api).mockResolvedValue({ sale: { id: 3 } } as never);
    await user.click(screen.getByRole('button', { name: 'Mixto' }));
    fireEvent.change(screen.getByLabelText('Efectivo (MXN)'), { target: { value: '100' } });
    fireEvent.change(screen.getByLabelText('Tarjeta (MXN)'), { target: { value: '70.50' } });
    fireEvent.change(screen.getByLabelText('Efectivo recibido'), { target: { value: '200' } });
    expect(screen.getByText('$100.00', { selector: 'b' })).toBeTruthy();
    await user.click(screen.getByRole('button', { name: /Confirmar pago/ }));
    expect(vi.mocked(api).mock.calls[0][1]).toMatchObject({
      tenderedCents: 20000,
      payments: [
        { method: 0, amountCents: 10000 },
        { method: 1, amountCents: 7050 },
      ],
    });
  });
  it('does not submit or add products when a scanner sends Enter into the payment input', () => {
    setup();
    const input = screen.getByLabelText('Efectivo recibido');
    for (const key of '7500000000001') fireEvent.keyDown(input, { key });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(api).not.toHaveBeenCalled();
    expect((input as HTMLInputElement).value).toBe('');
  });
});
