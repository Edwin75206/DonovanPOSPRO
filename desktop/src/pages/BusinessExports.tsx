import { useState } from 'react';
import { Check, Download, FileSpreadsheet } from 'lucide-react';
import { Button, Failure, PageTitle, useOperation } from '@/components/common';
import { api, type BusinessExportScope } from '@/lib/api';

const exportsByArea: { scope: BusinessExportScope; title: string; detail: string }[] = [
  {
    scope: 'salesToday',
    title: 'Ventas de hoy',
    detail: 'Ventas de hoy con sus partidas y pagos.',
  },
  { scope: 'sales', title: 'Todas las ventas', detail: 'Historial de ventas, partidas y pagos.' },
  { scope: 'customers', title: 'Clientes', detail: 'Directorio, límites de crédito y saldos.' },
  {
    scope: 'suppliers',
    title: 'Proveedores',
    detail: 'Datos de contacto y estado de cada proveedor.',
  },
  {
    scope: 'inventory',
    title: 'Productos e inventario',
    detail: 'Precios, existencias y movimientos.',
  },
  { scope: 'quotes', title: 'Cotizaciones', detail: 'Propuestas, vigencias y partidas cotizadas.' },
  { scope: 'purchases', title: 'Compras', detail: 'Compras recibidas y detalle de partidas.' },
  { scope: 'returns', title: 'Devoluciones', detail: 'Productos devueltos y reembolsos.' },
  { scope: 'cash', title: 'Caja', detail: 'Aperturas, cierres y movimientos de efectivo.' },
  { scope: 'credit', title: 'Crédito', detail: 'Cargos, abonos y vencimientos.' },
];

export function BusinessExports() {
  const operation = useOperation();
  const [lastFile, setLastFile] = useState('');
  const exportScope = (scope: BusinessExportScope) => {
    setLastFile('');
    void operation.run(
      () => api('exportBusiness', { scope }),
      (result) => {
        if (result) setLastFile(result.path);
      },
    );
  };
  return (
    <main className="page-scroll business-exports">
      <PageTitle
        title="Exportar a Excel"
        description="Elige qué información guardar y en qué carpeta."
      />
      <section className="panel diagnostic-summary" aria-labelledby="export-entire-business">
        <div>
          <h2 id="export-entire-business">Toda la base del negocio</h2>
          <p>
            Guarda la información comercial en un archivo con hojas por área. Para restaurar el
            negocio, crea un respaldo en Administración de datos.
          </p>
        </div>
        <Button disabled={operation.busy} onClick={() => exportScope('all')}>
          <FileSpreadsheet size={17} />
          Exportar toda la base a Excel
        </Button>
      </section>
      <Failure error={operation.error} />
      {operation.busy && (
        <p className="muted" role="status">
          Preparando el archivo de Excel…
        </p>
      )}
      {lastFile && (
        <div className="notice backup-success" role="status">
          <Check size={19} />
          <div>
            <strong>Archivo de Excel guardado</strong>
            <p className="file-location">{lastFile}</p>
          </div>
        </div>
      )}
      <h2 className="export-area-heading">Exportaciones por área</h2>
      <div className="export-area-grid">
        {exportsByArea.map(({ scope, title, detail }) => (
          <section className="panel export-area" key={scope}>
            <div>
              <h3>{title}</h3>
              <p>{detail}</p>
            </div>
            <Button
              variant="outline"
              disabled={operation.busy}
              aria-label={`Exportar ${title.toLocaleLowerCase('es-MX')}`}
              onClick={() => exportScope(scope)}
            >
              <Download size={15} />
              Exportar
            </Button>
          </section>
        ))}
      </div>
    </main>
  );
}
