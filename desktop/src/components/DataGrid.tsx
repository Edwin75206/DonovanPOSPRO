import { useMemo, type ReactNode } from 'react';
import {
  useTable,
  tableFeatures,
  rowSortingFeature,
  rowPaginationFeature,
  createSortedRowModel,
  createPaginatedRowModel,
  sortFns,
  type ColumnDef,
  type SortingState,
} from '@tanstack/react-table';
import { ArrowUpDown, ArrowUp, ArrowDown, ChevronLeft, ChevronRight } from 'lucide-react';
import { Button, Empty } from './common';
const features = tableFeatures({
  rowSortingFeature,
  rowPaginationFeature,
  sortedRowModel: createSortedRowModel(),
  paginatedRowModel: createPaginatedRowModel(),
  sortFns,
});
export interface GridColumn<T> {
  id: string;
  label: string;
  value: (row: T) => string | number | null | undefined;
  render?: (row: T) => ReactNode;
  align?: 'left' | 'right';
  sortable?: boolean;
}
export interface GridServer {
  page: number;
  pageSize: number;
  total: number;
  onPage: (page: number) => void;
  sorting?: SortingState;
  onSorting?: (sorting: SortingState) => void;
}
export function DataGrid<T extends { id?: number | string }>({
  rows,
  columns,
  server,
  emptyTitle = 'No hay registros para mostrar.',
  caption,
}: {
  rows: T[];
  columns: GridColumn<T>[];
  server?: GridServer;
  emptyTitle?: string;
  caption: string;
}) {
  const definitions = useMemo<ColumnDef<typeof features, T>[]>(
    () =>
      columns.map((column) => ({
        id: column.id,
        accessorFn: column.value,
        header: column.label,
        enableSorting: column.sortable !== false,
        cell: (info) =>
          column.render ? column.render(info.row.original) : String(info.getValue() ?? '—'),
      })),
    [columns],
  );
  const table = useTable({
    features,
    data: rows,
    columns: definitions,
    getRowId: (row, index) => String(row.id ?? index),
    initialState: { pagination: { pageIndex: 0, pageSize: 25 } },
    manualPagination: !!server,
    manualSorting: !!server?.onSorting,
    rowCount: server?.total,
    enableMultiSort: false,
    ...(server
      ? {
          state: {
            pagination: { pageIndex: server.page, pageSize: server.pageSize },
            ...(server.sorting ? { sorting: server.sorting } : {}),
          },
          onPaginationChange: (update) => {
            const current = { pageIndex: server.page, pageSize: server.pageSize };
            const next = typeof update === 'function' ? update(current) : update;
            server.onPage(next.pageIndex);
          },
          ...(server.onSorting
            ? {
                onSortingChange: (update: SortingState | ((state: SortingState) => SortingState)) =>
                  server.onSorting?.(
                    typeof update === 'function' ? update(server.sorting ?? []) : update,
                  ),
              }
            : {}),
        }
      : {}),
  });
  return (
    <div className="data-grid">
      <div className="table-wrap">
        <table>
          <caption className="sr-only">{caption}</caption>
          <thead>
            {table.getHeaderGroups().map((group) => (
              <tr key={group.id}>
                {group.headers.map((header) => {
                  const definition = columns.find((c) => c.id === header.column.id);
                  const sorted = header.column.getIsSorted();
                  return (
                    <th
                      key={header.id}
                      className={definition?.align === 'right' ? 'numeric' : ''}
                      aria-sort={
                        sorted === 'asc' ? 'ascending' : sorted === 'desc' ? 'descending' : 'none'
                      }
                    >
                      {header.column.getCanSort() ? (
                        <button
                          className="sort-heading"
                          onClick={header.column.getToggleSortingHandler()}
                        >
                          <table.FlexRender header={header} />
                          {sorted === 'asc' ? (
                            <ArrowUp size={13} />
                          ) : sorted === 'desc' ? (
                            <ArrowDown size={13} />
                          ) : (
                            <ArrowUpDown size={12} />
                          )}
                        </button>
                      ) : (
                        <table.FlexRender header={header} />
                      )}
                    </th>
                  );
                })}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <tr key={row.id}>
                {row.getAllCells().map((cell) => (
                  <td
                    key={cell.id}
                    className={
                      columns.find((c) => c.id === cell.column.id)?.align === 'right'
                        ? 'numeric'
                        : ''
                    }
                  >
                    <table.FlexRender cell={cell} />
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
        {rows.length === 0 && <Empty title={emptyTitle} />}
      </div>
      <div className="grid-pagination">
        <span>
          {table.getRowCount()} registros · Página {table.state.pagination.pageIndex + 1} de{' '}
          {Math.max(1, table.getPageCount())}
        </span>
        <div>
          <Button
            variant="outline"
            onClick={() => table.previousPage()}
            disabled={!table.getCanPreviousPage()}
          >
            <ChevronLeft size={14} />
            Anterior
          </Button>
          <Button
            variant="outline"
            onClick={() => table.nextPage()}
            disabled={!table.getCanNextPage()}
          >
            Siguiente
            <ChevronRight size={14} />
          </Button>
        </div>
      </div>
    </div>
  );
}
