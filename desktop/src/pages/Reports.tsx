import { useMemo, useState } from 'react';
import {
  ChartNoAxesCombined,
  FileDown,
  FileSpreadsheet,
  RefreshCw,
  Search,
  X,
  Info,
} from 'lucide-react';
import { toast } from 'sonner';
import {
  api,
  type Bootstrap,
  type BusinessReport,
  type ReportQuery,
  type DocumentRequest,
} from '@/lib/api';
import { useResource } from '@/lib/useResource';
import { useVisual } from '@/lib/appearance';
import { today, money, quantity, units, dateTime } from '@/lib/utils';
import { Button, Busy, Failure, Field, PageTitle, useOperation } from '@/components/common';
import { Chart } from '@/components/Chart';
import { DataGrid, type GridColumn } from '@/components/DataGrid';
import { DocumentDialog } from '@/components/DocumentViewer';

const kinds = [
  ['daily', 'Ventas por día'],
  ['sales', 'Detalle de ventas'],
  ['products', 'Ventas por producto'],
  ['categories', 'Ventas por categoría'],
  ['users', 'Ventas por usuario'],
  ['methods', 'Cobros por método'],
  ['profit', 'Costos y ganancias'],
  ['inventory', 'Inventario actual'],
  ['purchases', 'Compras'],
  ['receivables', 'Cuentas por cobrar'],
  ['credits', 'Movimientos de crédito'],
  ['cash', 'Jornadas de caja'],
  ['cashMovements', 'Movimientos de caja'],
  ['returns', 'Devoluciones'],
] as const;
type Row = BusinessReport['rows'][number];
const day = (value: string) =>
  new Date(`${value.slice(0, 10)}T12:00:00`).toLocaleDateString('es-MX', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  });
function display(value: string | number | null, column: BusinessReport['columns'][number]) {
  if (value === null || value === undefined)
    return column.key.toLowerCase().includes('cost') ? 'Sin costo' : 'Pendiente';
  if (column.kind === 'money') return money(Number(value));
  if (column.kind === 'quantity') return quantity(Number(value));
  if (column.kind === 'unit') return units[Number(value)] ?? '';
  if (column.kind === 'date')
    return String(value).length === 10 ? day(String(value)) : dateTime(String(value));
  return String(value);
}
export function Reports({ data, filters }: { data: Bootstrap; filters: Record<string, unknown> }) {
  const [query, setQuery] = useState<ReportQuery>(() => ({
    kind: kinds.some(([kind]) => kind === filters.kind) ? String(filters.kind) : 'daily',
    from: typeof filters.from === 'string' ? filters.from : `${today().slice(0, 7)}-01`,
    to: typeof filters.to === 'string' ? filters.to : today(),
    search: '',
  }));
  const [document, setDocument] = useState<DocumentRequest | null>(null);
  const resource = useResource(
    () => api('report', query),
    [query.kind, query.from, query.to, query.search],
  );
  const exportOp = useOperation();
  const report = resource.data;
  const visual = useVisual();
  const current = query.kind === 'inventory' || query.kind === 'receivables';
  const ready =
    !!report &&
    !resource.busy &&
    !resource.error &&
    report.kind === query.kind &&
    report.from === query.from &&
    report.to === query.to &&
    report.search === query.search.trim();
  const columns = useMemo<GridColumn<Row>[]>(
    () =>
      report?.columns.map((column) => ({
        id: column.key,
        label: column.label,
        value: (r) => r[column.key],
        align:
          column.kind === 'money' || column.kind === 'quantity' || column.kind === 'integer'
            ? 'right'
            : 'left',
        render: (r) => (
          <span className={r[column.key] === null ? 'muted' : undefined}>
            {display(r[column.key], column)}
          </span>
        ),
      })) ?? [],
    [report?.columns],
  );
  const chart = report?.chart;
  const option = useMemo(() => {
    const values = chart?.values ?? [];
    const axisMoney = (v: number) =>
      '$' +
      new Intl.NumberFormat('es-MX', { notation: 'compact', maximumFractionDigits: 1 }).format(v);
    const category = {
      type: 'category' as const,
      data: values.map((p) => (chart?.daily ? day(p.name) : p.name)),
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: {
        color: visual.tokens['--subtle'],
        hideOverlap: true,
        ...(!chart?.daily ? { width: 160, overflow: 'truncate' as const } : {}),
      },
    };
    const numeric = {
      type: 'value' as const,
      axisLabel: { color: visual.tokens['--subtle'], formatter: axisMoney },
      splitLine: { lineStyle: { color: visual.tokens['--border'], type: 'dashed' as const } },
    };
    return {
      tooltip: {
        trigger: 'axis' as const,
        renderMode: 'richText' as const,
        confine: true,
        valueFormatter: (value: unknown) => money(Number(value) * 100),
      },
      grid: { left: chart?.daily ? 66 : 180, right: 36, top: 18, bottom: 40 },
      xAxis: chart?.daily ? category : numeric,
      yAxis: chart?.daily ? numeric : { ...category, inverse: true },
      series: [
        {
          name: chart?.title,
          type: chart?.daily ? ('line' as const) : ('bar' as const),
          smooth: 0.15,
          barMaxWidth: 22,
          showSymbol: values.length < 15,
          symbolSize: 7,
          data: values.map((p) => p.valueCents / 100),
          lineStyle: { width: 3 },
          itemStyle: { borderRadius: 3 },
          ...(chart?.daily ? { areaStyle: { opacity: 0.06 } } : {}),
        },
      ],
    };
  }, [chart, visual.tokens]);
  function period(days: number | null) {
    const end = today();
    const start = new Date();
    if (days === null) start.setDate(1);
    else start.setDate(start.getDate() - days + 1);
    const from = `${start.getFullYear()}-${String(start.getMonth() + 1).padStart(2, '0')}-${String(start.getDate()).padStart(2, '0')}`;
    setQuery((q) => ({ ...q, from, to: end }));
  }
  return (
    <main className="page-scroll page-container records-page reports-page">
      <PageTitle
        eyebrow="DECISIONES CON RESPALDO"
        title="Conoce tus resultados."
        description="Explora las cifras, encuentra el detalle y llévate un reporte listo para compartir."
        actions={
          <>
            <Button
              variant="outline"
              aria-label="Actualizar reporte"
              onClick={resource.reload}
              disabled={resource.busy || exportOp.busy}
            >
              <RefreshCw size={17} />
            </Button>
            <Button
              variant="outline"
              disabled={!ready || exportOp.busy}
              onClick={() => setDocument({ kind: 'report', id: 0, report: { ...query } })}
            >
              <FileDown size={17} />
              Ver PDF
            </Button>
            <Button
              disabled={!ready || exportOp.busy}
              onClick={() =>
                void exportOp.run(
                  () => api('exportReport', query),
                  (result) => {
                    if (result)
                      toast.success('Reporte de Excel guardado', {
                        description: result.path,
                        duration: 9000,
                      });
                  },
                )
              }
            >
              <FileSpreadsheet size={17} />
              {exportOp.busy ? 'Guardando…' : 'Exportar Excel'}
            </Button>
          </>
        }
      />
      <section className="surface report-filters" aria-label="Filtros del reporte">
        <div className="report-filter-row">
          <Field label="Qué quieres revisar">
            <select
              value={query.kind}
              onChange={(e) => setQuery((q) => ({ ...q, kind: e.target.value, search: '' }))}
              disabled={exportOp.busy}
            >
              {kinds.map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Desde">
            <input
              type="date"
              min="1900-01-01"
              max="9998-12-31"
              value={query.from}
              disabled={current || exportOp.busy}
              onChange={(e) => setQuery((q) => ({ ...q, from: e.target.value }))}
            />
          </Field>
          <Field label="Hasta">
            <input
              type="date"
              min="1900-01-01"
              max="9998-12-31"
              value={query.to}
              disabled={current || exportOp.busy}
              onChange={(e) => setQuery((q) => ({ ...q, to: e.target.value }))}
            />
          </Field>
          <div className="records-search report-search">
            <Search size={18} />
            <input
              aria-label="Buscar dentro del reporte"
              placeholder="Buscar en este reporte…"
              value={query.search}
              maxLength={120}
              disabled={exportOp.busy}
              onChange={(e) => setQuery((q) => ({ ...q, search: e.target.value }))}
            />
            {query.search && (
              <button
                type="button"
                className="icon-button"
                aria-label="Quitar búsqueda"
                onClick={() => setQuery((q) => ({ ...q, search: '' }))}
              >
                <X size={15} />
              </button>
            )}
          </div>
        </div>
        <div className="report-presets">
          {current ? (
            <span>Este reporte muestra el saldo actual del negocio.</span>
          ) : (
            <>
              <span>Ir a</span>
              <Button variant="ghost" onClick={() => period(1)} disabled={exportOp.busy}>
                Hoy
              </Button>
              <Button variant="ghost" onClick={() => period(7)} disabled={exportOp.busy}>
                7 días
              </Button>
              <Button variant="ghost" onClick={() => period(null)} disabled={exportOp.busy}>
                Este mes
              </Button>
            </>
          )}
          <small>
            MXN ·{' '}
            {report && !resource.busy
              ? `Actualizado ${dateTime(report.generatedAt)}`
              : 'Consultando información local'}
          </small>
        </div>
      </section>
      <Failure error={exportOp.error} />
      {resource.busy ? (
        <Busy>Preparando tu reporte…</Busy>
      ) : resource.error ? (
        <Failure error={resource.error} retry={resource.reload} />
      ) : (
        report && (
          <>
            <div className="report-metrics">
              {report.metrics.map((metric) => (
                <article key={metric.label}>
                  <span>{metric.label}</span>
                  <strong>
                    {metric.value === null
                      ? 'Pendiente'
                      : metric.kind === 'money'
                        ? money(metric.value)
                        : metric.value.toLocaleString('es-MX')}
                  </strong>
                  {metric.value === null && <small>Hay productos sin costo registrado.</small>}
                </article>
              ))}
            </div>
            <div className="report-scope">
              <Info size={19} />
              <p>
                {report.scope}
                {report.search && (
                  <>
                    {' '}
                    <strong>Los totales corresponden a la búsqueda “{report.search}”.</strong>
                  </>
                )}
              </p>
            </div>
            {chart && chart.values.length > 0 && (
              <section className="surface report-graph">
                <div className="report-section-title">
                  <div>
                    <ChartNoAxesCombined size={20} />
                    <h2>{chart.title}</h2>
                  </div>
                  <span>
                    {chart.daily
                      ? 'Selecciona un día para revisar sus cifras.'
                      : 'Hasta 12 resultados · Selecciona una barra para filtrar.'}
                  </span>
                </div>
                <div
                  style={{
                    height: chart.daily
                      ? '19rem'
                      : `${Math.max(16, Math.min(28, chart.values.length * 2.3))}rem`,
                  }}
                >
                  <Chart
                    option={option}
                    label={`${chart.title}. ${report.scope}`}
                    onSelect={(index) => {
                      const point = chart.values[index];
                      if (!point) return;
                      if (chart.daily)
                        setQuery((q) => ({ ...q, from: point.name, to: point.name, search: '' }));
                      else setQuery((q) => ({ ...q, search: point.name.slice(0, 120) }));
                    }}
                  />
                </div>
              </section>
            )}
            <section className="surface report-detail">
              <div className="report-section-title">
                <h2>{report.title}</h2>
                <span>
                  {report.rows.length.toLocaleString('es-MX')} filas · Ordena desde cada encabezado
                </span>
              </div>
              <DataGrid
                key={JSON.stringify(query)}
                rows={report.rows}
                columns={columns}
                caption={report.title}
                emptyTitle="No hay registros con esta selección."
              />
            </section>
          </>
        )
      )}
      {document && (
        <DocumentDialog request={document} data={data} onClose={() => setDocument(null)} />
      )}
    </main>
  );
}
