import { useEffect, useMemo, useState } from 'react';
import { motion } from 'motion/react';
import {
  ArrowDownLeft,
  ArrowUpRight,
  ArrowRight,
  ChartNoAxesCombined,
  Wallet,
  Users,
  Package,
  DatabaseBackup,
  ShieldCheck,
  RefreshCw,
  ReceiptText,
  Plus,
  CalendarDays,
  Activity,
  ShoppingBag,
} from 'lucide-react';
import { api, type Bootstrap, type DashboardData, type UiFailure, normalizeError } from '@/lib/api';
import { useVisual } from '@/lib/appearance';
import { money, quantity, today, units } from '@/lib/utils';
import { Chart } from '@/components/Chart';
import { Busy, Button, Empty, Failure, PageTitle, Modal, Field } from '@/components/common';
export type Navigate = (page: string, filters?: Record<string, unknown>) => void;
const dateLabel = (date: string) =>
  new Date(`${date.slice(0, 10)}T12:00:00`).toLocaleDateString('es-MX', {
    day: 'numeric',
    month: 'short',
  });
const dateAgo = (days: number) => {
  const d = new Date();
  d.setDate(d.getDate() - days);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
};
const axisMoney = (value: number) =>
  '$' +
  new Intl.NumberFormat('es-MX', { notation: 'compact', maximumFractionDigits: 1 }).format(value);
export function Dashboard({ data, onNavigate }: { data: Bootstrap; onNavigate: Navigate }) {
  const [range, setRange] = useState({ from: dateAgo(6), to: today() });
  const [draft, setDraft] = useState(range);
  const [custom, setCustom] = useState(false);
  const [report, setReport] = useState<DashboardData | null>(null);
  const [error, setError] = useState<UiFailure | null>(null);
  const [refresh, setRefresh] = useState(0);
  const visual = useVisual();
  useEffect(() => {
    let current = true;
    setReport(null);
    setError(null);
    void api('dashboard', range)
      .then((r) => {
        if (current) setReport(r);
      })
      .catch((e) => {
        if (current) setError(normalizeError(e));
      });
    return () => {
      current = false;
    };
  }, [range, refresh]);
  const lineOption = useMemo(
    () => ({
      tooltip: {
        trigger: 'axis' as const,
        renderMode: 'richText' as const,
        valueFormatter: (value: unknown) => money(Number(value) * 100),
      },
      grid: { left: 56, right: 20, top: 20, bottom: 36 },
      xAxis: {
        type: 'category' as const,
        data: report?.trend.map((p) => dateLabel(p.date)),
        axisLine: { lineStyle: { color: visual.tokens['--border'] } },
        axisTick: { show: false },
        axisLabel: { color: visual.tokens['--subtle'], hideOverlap: true },
      },
      yAxis: {
        type: 'value' as const,
        axisLabel: { color: visual.tokens['--subtle'], formatter: axisMoney },
        splitLine: { lineStyle: { color: visual.tokens['--border'], type: 'dashed' as const } },
      },
      series: [
        {
          name: 'Ventas netas',
          type: 'line' as const,
          smooth: 0.2,
          symbol: 'circle',
          symbolSize: 6,
          showSymbol: (report?.trend.length ?? 0) < 10,
          data: report?.trend.map((p) => p.totalCents / 100),
          lineStyle: { width: 3 },
          areaStyle: { color: visual.tokens['--chart-1'], opacity: 0.07 },
          emphasis: { focus: 'series' as const },
        },
      ],
    }),
    [report, visual.tokens],
  );
  const methodsOption = useMemo(
    () => ({
      tooltip: {
        trigger: 'axis' as const,
        renderMode: 'richText' as const,
        valueFormatter: (value: unknown) => money(Number(value) * 100),
      },
      grid: { left: 99, right: 30, top: 15, bottom: 28 },
      xAxis: {
        type: 'value' as const,
        axisLabel: { color: visual.tokens['--subtle'], formatter: axisMoney },
        splitLine: { lineStyle: { color: visual.tokens['--border'], type: 'dashed' as const } },
      },
      yAxis: {
        type: 'category' as const,
        data: ['Efectivo', 'Tarjeta', 'Transferencia'],
        axisLabel: { color: visual.tokens['--subtle'] },
        axisLine: { show: false },
        axisTick: { show: false },
        inverse: true,
      },
      series: [
        {
          type: 'bar' as const,
          barMaxWidth: 19,
          data: [0, 1, 2].map((method, i) => ({
            value: (report?.methods.find((p) => p.method === method)?.totalCents ?? 0) / 100,
            itemStyle: {
              color: visual.tokens[`--chart-${i + 1}` as '--chart-1'],
              borderRadius: [0, 4, 4, 0],
            },
          })),
        },
      ],
    }),
    [report, visual.tokens],
  );
  const change =
    report && report.previousCents !== 0
      ? ((report.totalCents - report.previousCents) / Math.abs(report.previousCents)) * 100
      : null;
  return (
    <main className="page-scroll dashboard" data-tour="dashboard">
      <PageTitle
        eyebrow="PANORAMA DEL NEGOCIO"
        title={`Buen día, ${data.actor?.name.split(' · ')[0] ?? 'Administrador'}.`}
        description="La información que importa, a tiempo para decidir."
        actions={
          <>
            <Button
              variant="outline"
              onClick={() => setRefresh((v) => v + 1)}
              aria-label="Actualizar indicadores"
            >
              <RefreshCw size={15} />
            </Button>
            <Button onClick={() => onNavigate('pos')}>
              <Plus size={17} />
              Nueva venta
            </Button>
          </>
        }
      />
      <div className="dashboard-period">
        <div className="period-tabs">
          {[
            { label: 'Hoy', days: 0 },
            { label: '7 días', days: 6 },
            { label: '30 días', days: 29 },
          ].map((p) => (
            <button
              key={p.days}
              className={range.from === dateAgo(p.days) && range.to === today() ? 'active' : ''}
              onClick={() => setRange({ from: dateAgo(p.days), to: today() })}
            >
              {p.label}
            </button>
          ))}
          <button
            onClick={() => {
              setDraft(range);
              setCustom(true);
            }}
          >
            <CalendarDays size={14} />
            Elegir fechas
          </button>
        </div>
        <span>
          {dateLabel(range.from)} — {dateLabel(range.to)} · MXN
        </span>
      </div>
      <Failure error={error} retry={() => setRefresh((v) => v + 1)} />
      {!report && !error && <Busy>Consultando las operaciones del negocio…</Busy>}
      {report && (
        <>
          <section className="dashboard-metrics" data-tour="metrics">
            <motion.article
              className="executive-metric featured"
              initial={visual.motion ? { opacity: 0, y: 5 } : false}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: visual.duration }}
            >
              <div>
                <span>Ventas netas del periodo</span>
                <ChartNoAxesCombined size={19} />
              </div>
              <strong>{money(report.totalCents)}</strong>
              <p>
                <span className={`metric-change ${change !== null && change < 0 ? 'down' : ''}`}>
                  {change === null
                    ? 'Sin periodo comparable'
                    : `${change >= 0 ? '+' : ''}${change.toFixed(1)}%`}
                </span>
                {change !== null && <span>vs. periodo anterior</span>}
              </p>
              <small>
                {report.operations} operaciones · {money(report.returnsCents)} en devoluciones
              </small>
            </motion.article>
            <article className="executive-metric">
              <div>
                <span>Efectivo en caja</span>
                <Wallet size={18} />
              </div>
              <strong>{money(report.cashCents)}</strong>
              <p>
                <i className={report.cashSession ? 'status-dot green' : 'status-dot'} />
                {report.cashSession ? 'Caja abierta · saldo actual' : 'Caja cerrada'}
              </p>
              <button className="text-button secondary" onClick={() => onNavigate('cash')}>
                Revisar caja <ArrowRight size={13} />
              </button>
            </article>
            <article className="executive-metric">
              <div>
                <span>Pendiente por cobrar</span>
                <Users size={18} />
              </div>
              <strong>{money(report.receivableCents)}</strong>
              <p>Saldo actual de clientes</p>
              <button
                className="text-button secondary"
                onClick={() => onNavigate('customers', { tab: 'credit' })}
              >
                Ver cuentas <ArrowRight size={13} />
              </button>
            </article>
            <article className="executive-metric">
              <div>
                <span>Ganancia estimada</span>
                <ArrowUpRight size={18} />
              </div>
              <strong className={report.profitCents === null ? 'metric-unavailable' : ''}>
                {report.profitCents === null ? 'Costo pendiente' : money(report.profitCents)}
              </strong>
              <p>
                {report.profitCents === null
                  ? 'Hay productos sin costo registrado'
                  : 'Sin IVA · antes de gastos'}
              </p>
              <button
                className="text-button secondary"
                onClick={() => onNavigate('reports', { ...range, kind: 'profit' })}
              >
                Revisar detalle <ArrowRight size={13} />
              </button>
            </article>
          </section>
          <section className="dashboard-charts" data-tour="charts">
            <article className="panel trend-panel">
              <div className="panel-heading">
                <div>
                  <h2>El ritmo de tus ventas</h2>
                  <p>Importes netos por fecha de operación</p>
                </div>
                <span className="chart-key">
                  <i />
                  Ventas netas
                </span>
              </div>
              <Chart
                label={`Ventas netas desde ${range.from} hasta ${range.to}. Total ${money(report.totalCents)}.`}
                option={lineOption}
                onSelect={(i) => {
                  const day = report.trend[i]?.date;
                  if (day) onNavigate('reports', { kind: 'sales', from: day, to: day });
                }}
              />
              <div className="period-totals">
                <div>
                  <span>Hoy</span>
                  <b>{money(report.dayCents)}</b>
                </div>
                <div>
                  <span>Últimos 7 días</span>
                  <b>{money(report.weekCents)}</b>
                </div>
                <div>
                  <span>Mes en curso</span>
                  <b>{money(report.monthCents)}</b>
                </div>
              </div>
            </article>
            <article className="panel collections-panel">
              <div className="panel-heading">
                <div>
                  <h2>Cómo entra el dinero</h2>
                  <p>Cobros y abonos, menos reembolsos</p>
                </div>
                <Wallet size={18} />
              </div>
              <Chart
                label="Cobros netos por efectivo, tarjeta y transferencia."
                option={methodsOption}
              />
              <div className="credit-issued">
                <span>Crédito otorgado en el periodo</span>
                <b>{money(report.creditGrantedCents)}</b>
              </div>
              <small className="panel-note">
                El crédito se cuenta como venta; entra a caja cuando se abona en efectivo.
              </small>
            </article>
          </section>
          <section className="dashboard-lists">
            <article className="panel">
              <div className="panel-heading">
                <div>
                  <h2>Los más vendidos</h2>
                  <p>Por importe neto del periodo</p>
                </div>
                <button
                  className="text-button"
                  onClick={() => onNavigate('reports', { ...range, kind: 'products' })}
                >
                  Ver reporte <ArrowUpRight size={14} />
                </button>
              </div>
              {report.topProducts.length ? (
                <div className="rank-list">
                  {report.topProducts.slice(0, 5).map((p, i) => (
                    <button
                      key={`${p.productId}-${i}`}
                      onClick={() => onNavigate('inventory', { productId: p.productId })}
                    >
                      <span className="rank-number">{String(i + 1).padStart(2, '0')}</span>
                      <div>
                        <strong>{p.name}</strong>
                        <small>
                          {quantity(p.quantityUnits)} {units[p.unit]} vendidos netos
                        </small>
                      </div>
                      <b>{money(p.totalCents)}</b>
                    </button>
                  ))}
                </div>
              ) : (
                <Empty title="Tu primera venta empieza aquí">
                  Los productos vendidos aparecerán en esta lista.
                </Empty>
              )}
            </article>
            <article className="panel">
              <div className="panel-heading">
                <div>
                  <h2>Antes de que se terminen</h2>
                  <p>
                    {report.stockAlerts.lowCount} con stock bajo · {report.stockAlerts.emptyCount}{' '}
                    agotados
                  </p>
                </div>
                <button
                  className="text-button"
                  onClick={() => onNavigate('inventory', { status: 'low' })}
                >
                  Reabastecer <ArrowUpRight size={14} />
                </button>
              </div>
              {report.lowStock.length ? (
                <div className="stock-watch">
                  {report.lowStock.slice(0, 5).map((p) => (
                    <button key={p.id} onClick={() => onNavigate('inventory', { productId: p.id })}>
                      <span className={`alert-icon ${p.stockUnits === 0 ? 'empty-stock' : ''}`}>
                        <Package size={17} />
                      </span>
                      <div>
                        <strong>{p.name}</strong>
                        <small>
                          Mínimo {quantity(p.minimumUnits)} {units[p.unit]}
                        </small>
                      </div>
                      <b className={p.stockUnits === 0 ? 'negative' : ''}>
                        {p.stockUnits === 0
                          ? 'Agotado'
                          : `${quantity(p.stockUnits)} ${units[p.unit]}`}
                      </b>
                    </button>
                  ))}
                </div>
              ) : (
                <Empty title="Existencias en buen nivel">
                  No hay productos activos por debajo de su mínimo.
                </Empty>
              )}
            </article>
          </section>
          <section className="dashboard-lists">
            <article className="panel">
              <div className="panel-heading">
                <div>
                  <h2>Últimas ventas</h2>
                  <p>Consulta o reimprime en un paso</p>
                </div>
                <button className="text-button" onClick={() => onNavigate('sales')}>
                  Historial <ArrowUpRight size={14} />
                </button>
              </div>
              <div className="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th>Folio / cliente</th>
                      <th>Usuario</th>
                      <th className="numeric">Importe</th>
                    </tr>
                  </thead>
                  <tbody>
                    {report.recentSales.map((s) => (
                      <tr key={s.id}>
                        <td>
                          <button
                            className="table-link"
                            onClick={() => onNavigate('sales', { saleId: s.id })}
                          >
                            {s.folio}
                          </button>
                          <small className="cell-subtitle">{s.customerName}</small>
                        </td>
                        <td>{s.cashier}</td>
                        <td className="numeric">
                          {money(s.totalCents)}
                          {s.cancelled && (
                            <small className="cell-subtitle">Devuelta / cancelada</small>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {!report.recentSales.length && <Empty title="Aún no hay ventas" />}
              </div>
            </article>
            <article className="panel">
              <div className="panel-heading">
                <div>
                  <h2>Tu negocio, bajo control</h2>
                  <p>Datos y actividad reciente</p>
                </div>
                <ShieldCheck size={18} />
              </div>
              <button
                className={`backup-health ${report.lastBackup ? '' : 'pending'}`}
                onClick={() => onNavigate('settings', { tab: 'data' })}
              >
                <DatabaseBackup size={24} />
                <div>
                  <strong>
                    {report.lastBackup ? 'Último respaldo' : 'Tu primer respaldo está pendiente'}
                  </strong>
                  <small>
                    {report.lastBackup
                      ? report.lastBackup.createdAt
                      : 'Guarda una copia para recuperar tu negocio en otro equipo.'}
                  </small>
                </div>
                <ArrowRight size={17} />
              </button>
              <div className="database-health">
                <span>
                  <i className="status-dot green" />
                  Base comprobada al iniciar
                </span>
                <small>
                  {report.lastIntegrity ?? 'Sin comprobación'} ·{' '}
                  {(report.databaseBytes / 1024 / 1024).toFixed(1)} MB
                </small>
              </div>
              <div className="activity-list">
                {report.activity.slice(0, 4).map((a) => (
                  <div key={a.id}>
                    <Activity size={13} />
                    <div>
                      <strong>{a.action}</strong>
                      <small>
                        {a.userName ?? 'Sistema'} · {a.reference}
                      </small>
                    </div>
                    <time>{a.createdAt.slice(11, 16)}</time>
                  </div>
                ))}
              </div>
            </article>
          </section>
          <section className="dashboard-lists">
            <article className="panel">
              <div className="panel-heading">
                <div>
                  <h2>Ventas por categoría</h2>
                  <p>Importes netos, incluidas devoluciones del periodo</p>
                </div>
                <button
                  className="text-button"
                  onClick={() => onNavigate('reports', { ...range, kind: 'categories' })}
                >
                  Detalle <ArrowUpRight size={14} />
                </button>
              </div>
              <div className="category-performance">
                {report.categories.map((c, i) => (
                  <div key={c.name}>
                    <span>
                      <i
                        style={{
                          background: visual.tokens[`--chart-${(i % 5) + 1}` as '--chart-1'],
                        }}
                      />
                      {c.name}
                    </span>
                    <b>{money(c.totalCents)}</b>
                  </div>
                ))}
                {!report.categories.length && (
                  <Empty title="Todavía no hay actividad en este periodo" />
                )}
              </div>
            </article>
            <article className="panel">
              <div className="panel-heading">
                <div>
                  <h2>Entradas y movimientos</h2>
                  <p>Últimas compras y cambios de inventario</p>
                </div>
                <button className="text-button" onClick={() => onNavigate('purchases')}>
                  Compras <ArrowUpRight size={14} />
                </button>
              </div>
              <div className="activity-list">
                {report.recentPurchases.slice(0, 2).map((p) => (
                  <div key={`p${p.id}`}>
                    <ShoppingBag size={16} />
                    <div>
                      <strong>{p.supplierName}</strong>
                      <small>{p.folio}</small>
                    </div>
                    <b>{money(p.totalCents)}</b>
                  </div>
                ))}
                {report.stockMovements.slice(0, 4).map((m) => (
                  <div key={m.id}>
                    <Package size={15} />
                    <div>
                      <strong>{m.productName}</strong>
                      <small>{m.kind}</small>
                    </div>
                    <b>
                      {m.deltaUnits > 0 ? '+' : ''}
                      {quantity(m.deltaUnits)} {units[m.unit]}
                    </b>
                  </div>
                ))}
              </div>
            </article>
          </section>
          <div className="executive-footer">
            <span>{report.users.filter((u) => u.active).length} usuarios activos</span>
            <span>
              {report.lastCashClose
                ? `Último cierre: diferencia ${money((report.lastCashClose.countedCents ?? 0) - (report.lastCashClose.expectedCents ?? 0))}`
                : 'El primer cierre de caja aún está pendiente.'}
            </span>
            <span>Los indicadores usan información de esta base local.</span>
          </div>
        </>
      )}
      <Modal
        open={custom}
        onClose={() => setCustom(false)}
        title="Elige el periodo"
        description="Compara hasta 367 días de operación."
      >
        <form
          onSubmit={(e) => {
            e.preventDefault();
            setRange(draft);
            setCustom(false);
          }}
        >
          <div className="form-grid">
            <Field label="Desde">
              <input
                type="date"
                value={draft.from}
                onChange={(e) => setDraft({ ...draft, from: e.target.value })}
                required
              />
            </Field>
            <Field label="Hasta">
              <input
                type="date"
                value={draft.to}
                min={draft.from}
                onChange={(e) => setDraft({ ...draft, to: e.target.value })}
                required
              />
            </Field>
          </div>
          <Button className="wide" type="submit">
            Aplicar periodo
          </Button>
        </form>
      </Modal>
    </main>
  );
}
