// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import { api, type Product } from '@/lib/api';
import { BarcodeReaderTest } from './BarcodeReaderTest';

vi.mock('@/lib/api', async (original) => ({
  ...(await original<typeof import('@/lib/api')>()),
  api: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it('checks the exact code from a keyboard reader without creating a transaction', async () => {
  vi.mocked(api).mockResolvedValue({ name: 'Cemento gris 50 kg' } as Product);
  let time = 1000;
  vi.spyOn(performance, 'now').mockImplementation(() => (time += 5));
  render(<BarcodeReaderTest count={0} />);
  const input = screen.getByRole('textbox', { name: /^Verificación de lectura/ });
  for (const key of 'DEMO-001') {
    if (/[A-Z]/.test(key)) fireEvent.keyDown(input, { key: 'Shift' });
    fireEvent.keyDown(input, { key });
  }
  fireEvent.keyDown(input, { key: 'Tab' });

  expect(await screen.findByText('Cemento gris 50 kg')).toBeTruthy();
  expect(screen.getByText('Código recibido · DEMO-001')).toBeTruthy();
  expect(vi.mocked(api).mock.calls).toEqual([['productForCode', { code: 'DEMO-001' }]]);
});
