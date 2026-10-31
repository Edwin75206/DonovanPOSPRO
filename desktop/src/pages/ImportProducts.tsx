import { useState } from 'react';
import {
  FileSpreadsheet,
  FolderOpen,
  ArrowRight,
  ArrowLeft,
  Check,
  ShieldCheck,
  Download,
} from 'lucide-react';
import { toast } from 'sonner';
import { api, type ImportBook, type ImportOptions, type ImportPreview } from '@/lib/api';
import { money, quantity } from '@/lib/utils';
import { Button, Modal, Field, Failure, useOperation } from '@/components/common';
import { DataGrid } from '@/components/DataGrid';
const defaults = (): ImportOptions => ({ mapping: {}, mode: 'add', prices: 'keep', stock: 'keep' });
export function ImportProducts({
  onClose,
  onImported,
}: {
  onClose: () => void;
  onImported: () => void;
}) {
  const operation = useOperation();
  const [book, setBook] = useState<ImportBook | null>(null);
  const [options, setOptions] = useState<ImportOptions>(defaults);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [agreed, setAgreed] = useState(false);
  const [savedFile, setSavedFile] = useState('');
  const receive = (value: ImportBook | null) => {
    if (value) {
      setBook(value);
      setOptions({ ...defaults(), mapping: value.mapping });
      setPreview(null);
      setAgreed(false);
    }
  };
  const select = () => void operation.run(() => api('chooseImportFile'), receive);
  const close = () => {
    if (!operation.busy) onClose();
  };
  return (
    <Modal
      open
      onClose={close}
      title="Importar productos desde Excel"
      description={
        preview
          ? 'Revisa los cambios antes de guardarlos.'
          : book
            ? 'Relaciona tus columnas y elige qué información actualizar.'
            : 'Carga tu catálogo sin capturar los productos uno por uno.'
      }
      className="import-dialog"
    >
      <div className="import-step">
        <span className={!book ? 'active' : ''}>1. Archivo</span>
        <ArrowRight size={15} />
        <span className={book && !preview ? 'active' : ''}>2. Columnas y reglas</span>
        <ArrowRight size={15} />
        <span className={preview ? 'active' : ''}>3. Revisar y confirmar</span>
      </div>
      <Failure error={operation.error} />
      {!book ? (
        <div className="restore-start">
          <div className="large-feature-icon">
            <FileSpreadsheet size={32} />
          </div>
          <h3>Empieza con tu archivo o nuestra plantilla.</h3>
          <p>
            Libro .xlsx, hasta 20 MB y 50,000 productos. Los encabezados deben estar en la primera
            fila.
          </p>
          <Button onClick={select} disabled={operation.busy}>
            <FolderOpen size={17} />
            {operation.busy ? 'Leyendo el archivo…' : 'Seleccionar archivo de Excel'}
          </Button>
          <Button
            variant="outline"
            disabled={operation.busy}
            onClick={() =>
              void operation.run(
                () => api('exportProducts', { template: true }),
                (result) => {
                  if (result) {
                    setSavedFile(result.path);
                    toast.success('Plantilla guardada con instrucciones.');
                  }
                },
              )
            }
          >
            <Download size={17} />
            Guardar plantilla vacía
          </Button>
          {savedFile && <small className="file-location">{savedFile}</small>}
          <small>
            La importación lee valores. No ejecuta fórmulas, macros ni conexiones externas.
          </small>
        </div>
      ) : !preview ? (
        <>
          <div className="import-file">
            <FileSpreadsheet size={22} />
            <div>
              <strong>{book.fileName}</strong>
              <small>{book.rows.toLocaleString('es-MX')} filas para revisar</small>
            </div>
            <Button variant="outline" onClick={select} disabled={operation.busy}>
              Cambiar archivo
            </Button>
          </div>
          <Field label="Hoja de Excel">
            <select
              value={book.sheet}
              disabled={operation.busy}
              onChange={(e) =>
                void operation.run(() => api('importSheet', { sheet: e.target.value }), receive)
              }
            >
              {book.sheets.map((s) => (
                <option key={s}>{s}</option>
              ))}
            </select>
          </Field>
          <div className="import-rules">
            <Field label="Productos">
              <select
                value={options.mode}
                onChange={(e) =>
                  setOptions({ ...options, mode: e.target.value as ImportOptions['mode'] })
                }
              >
                <option value="add">Solo agregar productos nuevos</option>
                <option value="update">Solo actualizar claves existentes</option>
                <option value="upsert">Agregar y actualizar por clave</option>
              </select>
            </Field>
            <Field label="Precios, mayoreo y costo existentes">
              <select
                disabled={options.mode === 'add'}
                value={options.prices}
                onChange={(e) =>
                  setOptions({ ...options, prices: e.target.value as ImportOptions['prices'] })
                }
              >
                <option value="keep">Conservar los valores actuales</option>
                <option value="replace">Usar los valores de Excel</option>
              </select>
            </Field>
            <Field label="Existencias de productos existentes">
              <select
                disabled={options.mode === 'add'}
                value={options.stock}
                onChange={(e) =>
                  setOptions({ ...options, stock: e.target.value as ImportOptions['stock'] })
                }
              >
                <option value="keep">Conservar las existencias actuales</option>
                <option value="replace">Sustituir por las cantidades de Excel</option>
                <option value="add">Sumar las cantidades de Excel</option>
              </select>
            </Field>
          </div>
          <div className="notice">
            Los campos sin asignar conservan su valor al actualizar. Cada movimiento de existencia
            quedará registrado en el historial. Para productos nuevos se usan los valores asignados
            en Excel.
          </div>
          <h3>Relaciona las columnas</h3>
          <div className="import-mapping">
            {book.fields.map((field) => (
              <Field key={field.key} label={`${field.label}${field.key === 'sku' ? ' *' : ''}`}>
                <select
                  value={options.mapping[field.key] ?? ''}
                  onChange={(e) => {
                    const mapping = { ...options.mapping };
                    if (e.target.value === '') delete mapping[field.key];
                    else mapping[field.key] = Number(e.target.value);
                    setOptions({ ...options, mapping });
                  }}
                >
                  <option value="">Sin asignar</option>
                  {book.headers.map((h, i) => (
                    <option key={i} value={i}>
                      {i + 1}. {h}
                    </option>
                  ))}
                </select>
              </Field>
            ))}
          </div>
          <details className="import-sample">
            <summary>Ver las primeras filas del archivo</summary>
            <div className="table-wrap">
              <table className="data-table">
                <thead>
                  <tr>
                    <th>Fila</th>
                    {book.headers.map((h, i) => (
                      <th key={i}>{h}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {book.sample.map((row) => (
                    <tr key={row.row}>
                      <td>{row.row}</td>
                      {row.values.map((v, i) => (
                        <td key={i}>{v}</td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </details>
          <div className="dialog-actions">
            <Button variant="outline" disabled={operation.busy} onClick={close}>
              Cancelar
            </Button>
            <Button
              disabled={operation.busy || !Object.hasOwn(options.mapping, 'sku')}
              onClick={() =>
                void operation.run(
                  () => api('validateImport', options),
                  (p) => {
                    setPreview(p);
                    setAgreed(false);
                  },
                )
              }
            >
              {operation.busy ? 'Validando todas las filas…' : 'Validar y revisar'}
              <ArrowRight size={17} />
            </Button>
          </div>
        </>
      ) : (
        <>
          <div className="import-file">
            <ShieldCheck size={24} />
            <div>
              <strong>
                {preview.valid
                  ? 'Todas las filas son válidas'
                  : `${preview.errorCount} filas necesitan corrección`}
              </strong>
              <small>
                {book.fileName} · {preview.total.toLocaleString('es-MX')} filas revisadas
              </small>
            </div>
          </div>
          <div className="data-counts">
            {[
              [preview.added, 'altas'],
              [preview.updated, 'actualizaciones'],
              [preview.priceChanges, 'cambios de precio / costo'],
              [preview.stockChanges, 'movimientos de inventario'],
            ].map(([n, l]) => (
              <div key={l}>
                <strong>{n}</strong>
                <span>{l}</span>
              </div>
            ))}
          </div>
          {!preview.valid ? (
            <>
              <div className="notice warning">
                No se ha importado ninguna fila. Corrige el archivo y vuelve a seleccionarlo, o
                ajusta las columnas y reglas. Se muestran hasta 200 errores.
              </div>
              <DataGrid
                rows={preview.errors}
                caption="Errores del archivo de importación"
                columns={[
                  { id: 'row', label: 'Fila en Excel', value: (r) => r.row },
                  { id: 'error', label: 'Qué corregir', value: (r) => r.message },
                ]}
              />
              <Button variant="outline" onClick={select} disabled={operation.busy}>
                <FolderOpen size={16} />
                Seleccionar archivo corregido
              </Button>
            </>
          ) : (
            <>
              <p className="subtle">
                Vista previa de las primeras {preview.rows.length} filas. Los {preview.total}{' '}
                productos se procesarán juntos; si aparece un conflicto, no se aplicará ninguna
                fila.
              </p>
              <DataGrid
                rows={preview.rows}
                caption="Cambios propuestos para los productos"
                columns={[
                  { id: 'row', label: 'Fila', value: (r) => r.row },
                  {
                    id: 'name',
                    label: 'Producto',
                    value: (r) => r.name,
                    render: (r) => (
                      <div className="table-product">
                        <strong>{r.name}</strong>
                        <small>{r.sku}</small>
                      </div>
                    ),
                  },
                  { id: 'action', label: 'Acción', value: (r) => r.action },
                  {
                    id: 'price',
                    label: 'Precio',
                    value: (r) => r.priceCents,
                    render: (r) => (
                      <div>
                        {r.oldPriceCents !== null && r.oldPriceCents !== r.priceCents && (
                          <del>{money(r.oldPriceCents)}</del>
                        )}{' '}
                        {money(r.priceCents)}
                      </div>
                    ),
                    align: 'right',
                  },
                  {
                    id: 'stock',
                    label: 'Existencia',
                    value: (r) => r.stockUnits,
                    render: (r) => (
                      <div>
                        {r.oldStockUnits !== null && r.oldStockUnits !== r.stockUnits && (
                          <del>{quantity(r.oldStockUnits)}</del>
                        )}{' '}
                        {quantity(r.stockUnits)}
                      </div>
                    ),
                    align: 'right',
                  },
                ]}
              />
              {(preview.priceChanges > 0 || preview.stockChanges > 0) && (
                <div className="notice warning">
                  Revisa especialmente los precios y existencias. Has seleccionado{' '}
                  {options.prices === 'replace'
                    ? 'actualizar precios, mayoreo y costos'
                    : 'conservar precios existentes'}{' '}
                  y{' '}
                  {options.stock === 'replace'
                    ? 'sustituir existencias'
                    : options.stock === 'add'
                      ? 'sumar existencias'
                      : 'conservar existencias de productos actuales'}
                  .
                </div>
              )}
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={agreed}
                  onChange={(e) => setAgreed(e.target.checked)}
                />
                Revisé la vista previa y autorizo estos cambios en el catálogo y el inventario.
              </label>
            </>
          )}
          <div className="dialog-actions">
            <Button
              variant="outline"
              disabled={operation.busy}
              onClick={() => {
                setPreview(null);
                setAgreed(false);
                operation.setError(null);
              }}
            >
              <ArrowLeft size={16} />
              Volver a las reglas
            </Button>
            {preview.valid && (
              <Button
                disabled={operation.busy || !agreed}
                onClick={() =>
                  void operation.run(
                    () => api('commitImport', { planId: preview.planId }),
                    (result) => {
                      toast.success(
                        `${result.added} productos agregados y ${result.updated} actualizados.`,
                      );
                      onImported();
                      onClose();
                    },
                  )
                }
              >
                <Check size={17} />
                {operation.busy ? 'Guardando todos los cambios…' : 'Confirmar importación'}
              </Button>
            )}
          </div>
        </>
      )}
    </Modal>
  );
}
