// @vitest-environment jsdom
import { beforeEach, expect, it, vi } from 'vitest';
const invoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
beforeEach(() => {
  vi.resetModules();
  invoke.mockReset();
});
it('discards old account data that arrives after a user change', async () => {
  const { api } = await import('./api');
  invoke.mockResolvedValueOnce({ uiContext: 'account-a' });
  await api('bootstrap');
  let finish!: (value: unknown) => void;
  invoke.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const old = api('products', { search: '', category: '', status: 'active', page: 0 }).catch(
    (error) => error,
  );
  invoke.mockResolvedValueOnce({ uiContext: null });
  await api('logout');
  invoke.mockResolvedValueOnce({ uiContext: 'account-b' });
  await api('login', { username: 'test-b', password: 'synthetic-test-only' });
  finish({ items: [{ name: 'Previous private catalog' }] });
  expect((await old).failure.code).toBe('SESSION_CHANGED');
  expect(invoke.mock.calls[1][1].context).toBe('account-a');
  invoke.mockResolvedValueOnce({ items: [] });
  await api('products', { search: '', category: '', status: 'active', page: 0 });
  expect(invoke.mock.calls.at(-1)?.[1].context).toBe('account-b');
});
it('an expired response from an old login cannot close the new login', async () => {
  const { api } = await import('./api');
  invoke.mockResolvedValueOnce({ uiContext: 'account-a' });
  await api('bootstrap');
  let fail!: (value: unknown) => void;
  invoke.mockImplementationOnce(
    () =>
      new Promise((_resolve, reject) => {
        fail = reject;
      }),
  );
  const old = api('cashOverview').catch((error) => error);
  invoke.mockResolvedValueOnce({ uiContext: null });
  await api('logout');
  invoke.mockResolvedValueOnce({ uiContext: 'account-b' });
  await api('login', { username: 'test-b', password: 'synthetic-test-only' });
  const ended = vi.fn();
  window.addEventListener('session-ended', ended);
  fail({
    code: 'SESSION_EXPIRED',
    message: 'Old session',
    incidentId: 'test-only',
    retryable: false,
  });
  await old;
  expect(ended).not.toHaveBeenCalled();
  window.removeEventListener('session-ended', ended);
});
