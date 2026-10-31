import { useEffect, useRef, useState, type RefObject } from 'react';
import {
  ArrowDown,
  ArrowUp,
  ChevronLeft,
  ChevronRight,
  CornerDownLeft,
  Search,
  X,
} from 'lucide-react';
import { api, normalizeError, type Product, type ProductPage, type UiFailure } from '@/lib/api';
import { money, units } from '@/lib/utils';
import { ScanBuffer, stockState } from '@/lib/register';
import { Button, Modal, Failure } from '@/components/common';
import { StockIndicator } from '@/components/StockIndicator';
import type { CartRow } from './Register';

export function ProductPicker({
  rows,
  onSelect,
  onClose,
  returnFocusRef,
}: {
  rows: CartRow[];
  onSelect: (product: Product) => boolean;
  onClose: () => void;
  returnFocusRef: RefObject<HTMLInputElement | null>;
}) {
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState('');
  const [brand, setBrand] = useState('');
  const [page, setPage] = useState(0);
  const [data, setData] = useState<ProductPage | null>(null);
  const [loading, setLoading] = useState(true);
  const [reading, setReading] = useState(false);
  const [error, setError] = useState<UiFailure | null>(null);
  const [selected, setSelected] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const results = useRef<HTMLDivElement>(null);
  const scanner = useRef(new ScanBuffer());
  const alive = useRef(true);
  const selecting = useRef(false);
  const query = useRef(0);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  useEffect(() => {
    const id = ++query.current;
    setLoading(true);
    setError(null);
    const timer = setTimeout(
      () => {
        void api('products', { search, category, brand, status: 'active', page })
          .then((value) => {
            if (alive.current && id === query.current) {
              setData(value);
              setSelected(0);
            }
          })
          .catch((e) => {
            if (alive.current && id === query.current) setError(normalizeError(e));
          })
          .finally(() => {
            if (alive.current && id === query.current) setLoading(false);
          });
      },
      search ? 90 : 0,
    );
    return () => {
      clearTimeout(timer);
      query.current++;
    };
  }, [search, category, brand, page]);
  useEffect(() => {
    results.current?.querySelector('[aria-selected=true]')?.scrollIntoView?.({ block: 'nearest' });
  }, [selected]);
  const choose = (product: Product) => {
    if (!alive.current || selecting.current) return;
    selecting.current = true;
    if (onSelect(product)) onClose();
    else {
      selecting.current = false;
      input.current?.focus();
    }
  };
  const exact = async (code: string) => {
    if (selecting.current) return;
    selecting.current = true;
    setReading(true);
    try {
      const product = await api('productForCode', { code });
      if (!alive.current) return;
      selecting.current = false;
      choose(product);
    } catch (e) {
      if (alive.current) {
        setError(normalizeError(e));
        input.current?.focus();
        input.current?.select();
      }
    } finally {
      selecting.current = false;
      if (alive.current) setReading(false);
    }
  };
  const change = (value: string) => {
    setSearch(value);
    setPage(0);
    setLoading(true);
  };
  return (
    <Modal
      open
      onClose={onClose}
      title="Buscar producto"
      description="Encuentra el artículo, revisa su existencia y agrégalo a la venta."
      className="product-picker-dialog"
      initialFocusRef={input}
      returnFocusRef={returnFocusRef}
    >
      <div className="picker-search">
        <Search size={21} />
        <input
          ref={input}
          role="combobox"
          aria-expanded={true}
          aria-haspopup="grid"
          aria-label="Buscar productos"
          placeholder="Nombre, clave, código, marca o medida…"
          autoComplete="off"
          maxLength={200}
          value={search}
          onChange={(e) => change(e.target.value)}
          aria-controls="product-results"
          aria-activedescendant={
            !loading && data?.items[selected] ? `pick-${data.items[selected].id}` : undefined
          }
          onKeyDown={(e) => {
            if (reading && e.key !== 'Escape') {
              e.preventDefault();
              return;
            }
            const code = scanner.current.key(
              e.key,
              performance.now(),
              true,
              e.ctrlKey || e.altKey || e.metaKey || e.nativeEvent.isComposing || e.repeat,
            );
            if (code) {
              e.preventDefault();
              e.stopPropagation();
              void exact(code);
              return;
            }
            if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
              e.preventDefault();
              setSelected((n) =>
                Math.max(
                  0,
                  Math.min((data?.items.length ?? 1) - 1, n + (e.key === 'ArrowDown' ? 1 : -1)),
                ),
              );
            }
            if (e.key === 'Enter') {
              e.preventDefault();
              if (!loading && data?.items[selected]) choose(data.items[selected]);
            }
          }}
          onBlur={() => scanner.current.reset()}
        />
        {search && (
          <button
            aria-label="Limpiar búsqueda"
            className="icon-button"
            onClick={() => {
              change('');
              input.current?.focus();
            }}
          >
            <X size={17} />
          </button>
        )}
        <kbd>F2</kbd>
      </div>
      <div className="picker-filters">
        <div className="category-strip" aria-label="Categorías de productos">
          <button
            aria-pressed={!category}
            className={!category ? 'active' : ''}
            onClick={() => {
              if (category) {
                setCategory('');
                setPage(0);
                setLoading(true);
              }
              input.current?.focus();
            }}
          >
            Todos
          </button>
          {data?.categories.map((c) => (
            <button
              key={c.name}
              aria-pressed={category === c.name}
              className={category === c.name ? 'active' : ''}
              onClick={() => {
                if (category !== c.name) {
                  setCategory(c.name);
                  setPage(0);
                  setLoading(true);
                }
                input.current?.focus();
              }}
            >
              {c.name}
            </button>
          ))}
        </div>
        {!!data?.brands?.length && (
          <label className="picker-brand">
            <span>Marca</span>
            <select
              aria-label="Filtrar por marca"
              value={brand}
              onChange={(e) => {
                setBrand(e.target.value);
                setPage(0);
                setLoading(true);
                input.current?.focus();
              }}
            >
              <option value="">Todas las marcas</option>
              {data.brands.map((b) => (
                <option key={b.name} value={b.name}>
                  {b.name}
                </option>
              ))}
            </select>
          </label>
        )}
      </div>
      <Failure error={error} />
      <div className="picker-result-meta" role="status">
        {reading
          ? 'Identificando código…'
          : loading
            ? 'Buscando…'
            : `${data?.total ?? 0} ${data?.total === 1 ? 'producto' : 'productos'}`}
        <span>Precios con IVA · Disponibilidad para esta venta</span>
      </div>
      <div className="picker-results" ref={results} aria-busy={loading || reading}>
        <table role="grid" aria-label="Resultados de productos" id="product-results">
          <thead>
            <tr>
              <th>Producto</th>
              <th>Existencia disponible</th>
              <th>Unidad</th>
              <th className="numeric">Precio</th>
              <th>
                <span className="sr-only">Seleccionar</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {data?.items.map((product, index) => {
              const reserved = rows.find((r) => r.product.id === product.id)?.quantityUnits ?? 0;
              const disabled =
                loading ||
                reading ||
                stockState(product, reserved).available <= 0 ||
                !product.active;
              return (
                <tr
                  key={product.id}
                  id={`pick-${product.id}`}
                  aria-selected={index === selected}
                  className={disabled && !loading ? 'unavailable' : ''}
                  onClick={() => {
                    if (!disabled) choose(product);
                  }}
                >
                  <td>
                    <strong>{product.name}</strong>
                    <small>
                      {product.sku}
                      {product.brand && ` · ${product.brand}`}
                      {product.size && ` · ${product.size}`}
                    </small>
                  </td>
                  <td>
                    <StockIndicator product={product} reserved={reserved} />
                  </td>
                  <td className="unit-cell">{units[product.unit]}</td>
                  <td className="numeric">
                    <b>{money(product.priceCents)}</b>
                  </td>
                  <td>
                    <button
                      disabled={disabled}
                      aria-label={`Agregar ${product.name}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        choose(product);
                      }}
                      className="picker-add"
                    >
                      <CornerDownLeft size={15} />
                      <span>Agregar</span>
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        {!loading && !data?.items.length && (
          <div className="picker-empty">
            <Search size={28} />
            <h3>No encontramos coincidencias</h3>
            <p>Prueba con otro nombre, medida o código, o cambia los filtros.</p>
          </div>
        )}
      </div>
      <div className="picker-footer">
        <span>
          <kbd>
            <ArrowUp size={12} />
            <ArrowDown size={12} />
          </kbd>{' '}
          Elegir <kbd>Enter</kbd> Agregar <kbd>Esc</kbd> Volver a la venta
        </span>
        {(data?.total ?? 0) > (data?.pageSize ?? 60) && (
          <div className="picker-pagination">
            <Button
              size="sm"
              variant="outline"
              disabled={loading || !page}
              aria-label="Página anterior de productos"
              onClick={() => {
                setPage(page - 1);
                setLoading(true);
              }}
            >
              <ChevronLeft size={16} />
            </Button>
            <span>
              {page + 1} / {Math.ceil((data?.total ?? 0) / (data?.pageSize ?? 60))}
            </span>
            <Button
              size="sm"
              variant="outline"
              disabled={loading || (page + 1) * (data?.pageSize ?? 60) >= (data?.total ?? 0)}
              aria-label="Página siguiente de productos"
              onClick={() => {
                setPage(page + 1);
                setLoading(true);
              }}
            >
              <ChevronRight size={16} />
            </Button>
          </div>
        )}
      </div>
    </Modal>
  );
}
