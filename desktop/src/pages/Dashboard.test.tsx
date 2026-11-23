// @vitest-environment jsdom
import { useState } from 'react';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { api, type Bootstrap, type DashboardData } from '@/lib/api';
import { Dashboard } from './Dashboard';

vi.mock('@/lib/api', async (original) => ({
  ...(await original<typeof import('@/lib/api')>()),
  api: vi.fn(),
}));
vi.mock('@/components/Chart', () => ({ Chart: () => null }));
vi.mock('@/lib/appearance', () => ({
  useVisual: () => ({ tokens: {}, motion: false, duration: 0 }),
}));

afterEach(cleanup);

test('keeps operating totals visible while collapsing and reopening dashboard detail', async () => {
  const report: DashboardData = {
    from: '2026-11-15',
    to: '2026-11-21',
    dayCents: 12000,
    weekCents: 12000,
    monthCents: 12000,
    grossCents: 12000,
    returnsCents: 0,
    totalCents: 12000,
    previousCents: 0,
    profitCents: 5000,
    operations: 1,
    dayOperations: 1,
    receivableCents: 0,
    creditGrantedCents: 0,
    cashCents: 12000,
    cashSession: null,
    lastCashClose: null,
    trend: [],
    methods: [],
    topProducts: [],
    categories: [],
    lowStock: [],
    stockAlerts: { emptyCount: 0, lowCount: 0 },
    recentSales: [],
    recentPurchases: [],
    activity: [],
    stockMovements: [],
    users: [],
    lastBackup: null,
    lastIntegrity: null,
    databaseBytes: 1024,
    schemaVersion: 5,
  };
  vi.mocked(api).mockResolvedValue(report);

  function View() {
    const [collapsed, setCollapsed] = useState(false);
    return (
      <Dashboard
        data={{} as Bootstrap}
        onNavigate={vi.fn()}
        collapsed={collapsed}
        onToggle={() => setCollapsed((value) => !value)}
      />
    );
  }

  await act(async () => {
    render(<View />);
  });
  expect(screen.getByRole('heading', { name: 'El ritmo de tus ventas' })).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Contraer Dashboard', expanded: true }));
  expect(screen.queryByRole('heading', { name: 'El ritmo de tus ventas' })).toBeNull();
  expect(screen.getByText('Ventas netas del periodo')).toBeTruthy();
  expect(screen.getByText('Efectivo en caja')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Expandir Dashboard', expanded: false }));
  expect(screen.getByRole('heading', { name: 'El ritmo de tus ventas' })).toBeTruthy();
  expect(api).toHaveBeenCalledTimes(1);
});
