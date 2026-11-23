// @vitest-environment jsdom
import { useState, type ComponentProps } from 'react';
import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { emit, TauriEvent } from '@tauri-apps/api/event';
import { clearMocks, mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { api, type Bootstrap } from '@/lib/api';
import { useLeaveGuard } from '@/lib/leaveGuard';
import { newProduct } from '@/pages/Inventory';
import type { Register } from '@/pages/Register';
import capability from '../src-tauri/capabilities/main.json';
import App from './App';

vi.mock('@/lib/api', async (original) => ({
  ...(await original<typeof import('@/lib/api')>()),
  api: vi.fn(),
}));
vi.mock('@/pages/Dashboard', () => ({ Dashboard: () => <h1>Inicio</h1> }));
vi.mock('@/pages/Register', async (original) => ({
  ...(await original<typeof import('@/pages/Register')>()),
  Register: RegisterFixture,
}));

function RegisterFixture({ setCart }: Pick<ComponentProps<typeof Register>, 'setCart'>) {
  const [editing, setEditing] = useState(false);
  const leaveGuard = useLeaveGuard(editing);
  return (
    <>
      <h1>Venta de mostrador</h1>
      <button
        onClick={() => setCart((cart) => ({
          ...cart,
          rows: [{ product: { ...newProduct(), id: 1 }, quantityUnits: 10000, wholesale: false }],
        }))}
      >
        Agregar al ticket
      </button>
      <button onClick={() => setEditing(true)}>Editar ficha</button>
      {leaveGuard}
    </>
  );
}

const windowCommand = vi.fn();
const requestClose = () => act(async () => {
  await emit(TauriEvent.WINDOW_CLOSE_REQUESTED);
});

beforeEach(async () => {
  mockWindows('main');
  mockIPC(windowCommand, { shouldMockEvents: true });
  vi.stubGlobal('matchMedia', (media: string) => ({
    media,
    matches: false,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
  const data: Bootstrap = {
    uiContext: null,
    needsSetup: false,
    actor: { id: 1, username: 'admin', name: 'Administración', role: 0 },
    settings: null,
    preferences: null,
    appearance: null,
    profile: { storeName: 'Construrama Chicontla', accent: '#1e7354', highlight: '#e8d7be' },
    version: '1.0',
    demo: false,
    locked: false,
    storageError: null,
  };
  vi.mocked(api).mockResolvedValue(data);
  render(<App />);
  await screen.findByRole('heading', { name: 'Inicio' });
});

afterEach(() => {
  cleanup();
  clearMocks();
  vi.unstubAllGlobals();
  vi.resetAllMocks();
});

test('F1 opens the register and F10 opens help without navigating through an open dialog', async () => {
  fireEvent.keyDown(window, { key: 'F10' });
  const help = await screen.findByRole('dialog', { name: 'Un apoyo para tu jornada.' });
  expect(within(help).getByText('Abrir Punto de venta')).toBeTruthy();
  fireEvent.keyDown(window, { key: 'F1' });
  expect(screen.queryByText('Venta de mostrador')).toBeNull();

  fireEvent.click(within(help).getByRole('button', { name: 'Cerrar' }));
  fireEvent.keyDown(window, { key: 'F1' });
  expect(await screen.findByRole('heading', { name: 'Venta de mostrador' })).toBeTruthy();
  expect(screen.queryByRole('dialog')).toBeNull();
});

test('the title-bar close request closes an idle window exactly once', async () => {
  expect(capability.permissions).toContain('core:window:allow-destroy');
  await requestClose();
  expect(windowCommand.mock.calls).toEqual([['plugin:window|destroy', { label: 'main' }]]);
});

test('a draft ticket keeps the window open unless the user confirms discarding it', async () => {
  fireEvent.keyDown(window, { key: 'F1' });
  fireEvent.click(await screen.findByRole('button', { name: 'Agregar al ticket' }));
  await requestClose();
  const dialog = await screen.findByRole('dialog', { name: 'Hay tickets sin guardar' });
  fireEvent.click(within(dialog).getByRole('button', { name: 'Volver al sistema' }));
  expect(windowCommand).not.toHaveBeenCalled();

  await requestClose();
  fireEvent.click(await screen.findByRole('button', { name: 'Salir y descartar borradores' }));
  expect(windowCommand.mock.calls).toEqual([['plugin:window|destroy', { label: 'main' }]]);
});

test('an unsaved edit keeps the window open until the user chooses to leave', async () => {
  fireEvent.keyDown(window, { key: 'F1' });
  fireEvent.click(await screen.findByRole('button', { name: 'Editar ficha' }));
  await requestClose();
  const dialog = await screen.findByRole('dialog', { name: 'Hay cambios sin guardar' });
  expect(windowCommand).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole('button', { name: 'Salir sin guardar' }));
  expect(windowCommand.mock.calls).toEqual([['plugin:window|destroy', { label: 'main' }]]);
});
