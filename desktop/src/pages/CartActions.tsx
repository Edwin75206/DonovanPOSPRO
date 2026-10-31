import { useState } from 'react';
import { Plus, Search, ArrowRight, Pause, FileText, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import { api, can, normalizeError, type Actor, type Contact, type HeldSale } from '@/lib/api';
import { money, dateTime, today } from '@/lib/utils';
import { useResource } from '@/lib/useResource';
import { Button, Modal, Field, Failure, useOperation } from '@/components/common';
import { DataGrid } from '@/components/DataGrid';
import { ContactEditor, newContact } from './Contacts';
import { freshCart, type Cart } from './Register';

export function CustomerPicker({
  actor,
  onClose,
  onSelect,
}: {
  actor: Actor;
  onClose: () => void;
  onSelect: (c: Contact | null) => void;
}) {
  const [search, setSearch] = useState('');
  const [create, setCreate] = useState(false);
  const customers = useResource(
    () => api('contacts', { supplier: false, search, archived: false }),
    [search],
  );
  return (
    <Modal
      open
      onClose={onClose}
      title="¿Para quién es la venta?"
      description="Busca a tu cliente o continúa con público en general."
      className="records-dialog"
    >
      <div className="records-toolbar">
        <div className="records-search">
          <Search size={18} />
          <input
            autoFocus
            placeholder="Nombre o teléfono…"
            aria-label="Buscar cliente para la venta"
            value={search}
            maxLength={300}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        {can(actor, 'credit') && (
          <Button variant="outline" onClick={() => setCreate(true)}>
            <Plus size={16} /> Nuevo cliente
          </Button>
        )}
      </div>
      <Button variant="outline" onClick={() => onSelect(null)}>
        Público en general
      </Button>
      <Failure error={customers.error} retry={customers.reload} />
      <DataGrid
        rows={customers.data ?? []}
        caption="Seleccionar cliente"
        emptyTitle={customers.busy ? 'Buscando…' : 'No encontramos ese cliente.'}
        columns={[
          {
            id: 'name',
            label: 'Cliente',
            value: (c) => c.name,
            render: (c) => (
              <div className="two-line">
                <strong>{c.name}</strong>
                <small>{c.phone || c.email}</small>
              </div>
            ),
          },
          ...(can(actor, 'credit')
            ? [
                {
                  id: 'credit',
                  label: 'Crédito disponible',
                  value: (c: Contact) => Math.max(0, c.creditLimitCents - (c.balanceCents ?? 0)),
                  render: (c: Contact) =>
                    money(Math.max(0, c.creditLimitCents - (c.balanceCents ?? 0))),
                  align: 'right' as const,
                },
              ]
            : []),
          {
            id: 'select',
            label: 'Elegir',
            value: (c) => c.id,
            sortable: false,
            render: (c) => (
              <Button
                size="sm"
                variant="outline"
                aria-label={`Seleccionar ${c.name}`}
                onClick={() => onSelect(c)}
              >
                Seleccionar <ArrowRight size={15} />
              </Button>
            ),
          },
        ]}
      />
      {create && (
        <ContactEditor contact={newContact()} onClose={() => setCreate(false)} onSaved={onSelect} />
      )}
    </Modal>
  );
}
export function SaveDraft({
  cart,
  kind,
  onClose,
  onSaved,
}: {
  cart: Cart;
  kind: 'held' | 'quote';
  onClose: () => void;
  onSaved: (result: { id: number; folio: string }) => void;
}) {
  const [label, setLabel] = useState(cart.customerName || 'Venta de mostrador');
  const [notes, setNotes] = useState(cart.editingQuote?.notes ?? '');
  const [validUntil, setValidUntil] = useState(() => {
    if (cart.editingQuote && cart.editingQuote.validUntil >= today())
      return cart.editingQuote.validUntil;
    const d = new Date();
    d.setDate(d.getDate() + 15);
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  });
  const [requestId] = useState(() => crypto.randomUUID());
  const op = useOperation();
  const items = cart.rows.map((r) => ({
    productId: r.product.id,
    quantityUnits: r.quantityUnits,
    wholesale: r.wholesale,
  }));
  return (
    <Modal
      open
      onClose={() => {
        if (!op.busy) onClose();
      }}
      title={
        kind === 'held'
          ? 'Continuamos más tarde.'
          : cart.editingQuote
            ? `Actualizar ${cart.sourceLabel}`
            : 'Una propuesta para tu cliente.'
      }
      description={
        kind === 'held'
          ? 'Guardaremos el carrito sin cobrar ni reservar existencias.'
          : cart.editingQuote
            ? 'Al guardar se sustituirán las partidas de esta propuesta. Se conserva su folio y no se modifica inventario.'
            : 'La cotización conserva los precios acordados y no modifica inventario.'
      }
    >
      <form
        className="record-form"
        onSubmit={(e) => {
          e.preventDefault();
          void op.run(
            async () => {
              if (kind === 'held') {
                const result = await api('saveHeld', {
                  requestId,
                  id: cart.heldSaleId,
                  label,
                  items,
                  customerId: cart.customerId,
                  discountParts: cart.discountParts,
                });
                return { id: result.id, folio: result.folio };
              }
              const result = await api('saveQuote', {
                requestId,
                id: cart.editingQuote?.id ?? 0,
                items,
                customerId: cart.customerId,
                discountParts: cart.discountParts,
                validUntil,
                notes,
              });
              return { id: result.quote.id, folio: result.quote.folio };
            },
            (result) => {
              toast.success(
                `${kind === 'held' ? 'Venta guardada' : cart.editingQuote ? 'Cotización actualizada' : 'Cotización creada'} · ${result.folio}`,
              );
              onSaved(result);
            },
          );
        }}
      >
        <fieldset disabled={op.busy}>
          {kind === 'held' ? (
            <Field label="Identifica esta venta">
              <input
                autoFocus
                maxLength={100}
                required
                value={label}
                onChange={(e) => setLabel(e.target.value)}
              />
            </Field>
          ) : (
            <>
              <Field label="Válida hasta">
                <input
                  autoFocus
                  type="date"
                  required
                  min={today()}
                  value={validUntil}
                  onChange={(e) => setValidUntil(e.target.value)}
                />
              </Field>
              <Field label="Observaciones">
                <textarea
                  value={notes}
                  maxLength={2000}
                  onChange={(e) => setNotes(e.target.value)}
                  placeholder="Entrega, condiciones o detalles para tu cliente…"
                />
              </Field>
            </>
          )}
          <p className="notice">
            {cart.customerName || 'Público en general'} · {cart.rows.length}{' '}
            {cart.rows.length === 1 ? 'partida' : 'partidas'}.
            {kind === 'held' && ' Al recuperarla se revisarán precios y existencias.'}
          </p>
        </fieldset>
        <Failure error={op.error} />
        <div className="dialog-actions">
          <Button type="button" variant="outline" disabled={op.busy} onClick={onClose}>
            Volver
          </Button>
          <Button type="submit" disabled={op.busy}>
            {kind === 'held' ? <Pause size={16} /> : <FileText size={16} />}{' '}
            {op.busy
              ? 'Guardando…'
              : kind === 'held'
                ? 'Guardar venta'
                : cart.editingQuote
                  ? 'Guardar cambios'
                  : 'Crear cotización'}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
export function HeldSales({
  actor,
  hasCart,
  onClose,
  onLoad,
}: {
  actor: Actor;
  hasCart: boolean;
  onClose: () => void;
  onLoad: (cart: Cart) => void;
}) {
  const list = useResource(() => api('heldSales'), []);
  const [selected, setSelected] = useState<HeldSale | null>(null);
  const [discard, setDiscard] = useState<HeldSale | null>(null);
  const [prepared, setPrepared] = useState<{
    cart: Cart;
    notices: string[];
    total: number | null;
  } | null>(null);
  const op = useOperation();
  const load = (row: HeldSale) => {
    setSelected(row);
    void op.run(async () => {
      const result = await api('heldSale', { id: row.id });
      const customer = result.sale.customerId
        ? await api('contact', { supplier: false, id: result.sale.customerId })
        : null;
      const permitted = can(actor, 'discount');
      const rows = result.rows.map((r) => ({ ...r, wholesale: permitted && r.wholesale }));
      const discountParts = permitted ? result.sale.discountParts : 0;
      const notices = [...result.notices];
      if (!permitted && (result.sale.discountParts > 0 || result.rows.some((r) => r.wholesale)))
        notices.push('Tu perfil actual recuperará esta venta a precio de menudeo, sin descuentos.');
      if (customer && !customer.active)
        notices.push(
          'El cliente está archivado. Recuperaremos el ticket como Público en general para que puedas elegir un cliente activo.',
        );
      const price = await api('price', {
        items: rows.map((r) => ({
          productId: r.product.id,
          quantityUnits: r.quantityUnits,
          wholesale: r.wholesale,
        })),
        discountParts,
      }).catch((error) => {
        if (normalizeError(error).code === 'PRODUCT_UNAVAILABLE') {
          notices.push(
            'Quita los productos archivados antes de cobrar. El importe se calculará al corregir el ticket.',
          );
          return null;
        }
        throw error;
      });
      return {
        cart: {
          ...freshCart(),
          rows,
          customerId: customer?.active ? customer.id : null,
          customerName: customer?.active ? customer.name : null,
          discountParts,
          heldSaleId: row.id,
          sourceLabel: row.folio,
        },
        notices,
        total: price?.totalCents ?? null,
      };
    }, setPrepared);
  };
  return (
    <Modal
      open
      onClose={() => {
        if (!op.busy) onClose();
      }}
      title="Ventas guardadas"
      description="Recupera una venta y sigue atendiendo donde te quedaste."
      className="records-dialog"
    >
      <Failure error={op.error ?? list.error} retry={list.reload} />
      {hasCart && (
        <p className="notice">
          Tu carrito tiene productos. Guárdalo o vacíalo antes de recuperar otra venta.
        </p>
      )}
      <DataGrid
        rows={list.data ?? []}
        caption="Ventas suspendidas"
        emptyTitle={list.busy ? 'Buscando ventas…' : 'No hay ventas en espera.'}
        columns={[
          {
            id: 'label',
            label: 'Venta',
            value: (s) => s.label,
            render: (s) => (
              <div className="two-line">
                <strong>{s.label}</strong>
                <small>
                  {s.folio} · {s.userName}
                </small>
              </div>
            ),
          },
          {
            id: 'date',
            label: 'Guardada',
            value: (s) => s.createdAt,
            render: (s) => dateTime(s.createdAt),
          },
          {
            id: 'total',
            label: 'Importe guardado',
            value: (s) => s.totalCents,
            render: (s) => money(s.totalCents),
            align: 'right',
          },
          {
            id: 'actions',
            label: 'Acciones',
            value: (s) => s.id,
            sortable: false,
            render: (s) => (
              <div className="row-actions">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={op.busy || hasCart}
                  onClick={() => load(s)}
                >
                  Recuperar
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={op.busy}
                  aria-label={`Descartar ${s.folio}`}
                  onClick={() => setDiscard(s)}
                >
                  <Trash2 size={15} />
                </Button>
              </div>
            ),
          },
        ]}
      />
      {prepared && (
        <Modal
          open
          onClose={() => setPrepared(null)}
          title={`Continuar ${selected?.folio}`}
          description="Revisamos el catálogo y actualizamos el importe."
        >
          <div className="account-summary">
            <div>
              <span>IMPORTE ACTUAL</span>
              <strong>
                {prepared.total === null ? 'Revisa los productos' : money(prepared.total)}
              </strong>
              <small>{prepared.cart.customerName || 'Público en general'}</small>
            </div>
          </div>
          {prepared.notices.map((n, i) => (
            <p className="notice" key={i}>
              {n}
            </p>
          ))}
          <Button disabled={hasCart} onClick={() => onLoad(prepared.cart)}>
            Recuperar con precios actuales <ArrowRight size={16} />
          </Button>
        </Modal>
      )}
      {discard && (
        <Modal
          open
          onClose={() => {
            if (!op.busy) setDiscard(null);
          }}
          title={`Descartar ${discard.folio}`}
          description="Se cerrará esta venta guardada. No hubo cobro ni salida de inventario."
        >
          <Failure error={op.error} />
          <div className="dialog-actions">
            <Button variant="outline" disabled={op.busy} onClick={() => setDiscard(null)}>
              Conservar
            </Button>
            <Button
              variant="destructive"
              disabled={op.busy}
              onClick={() =>
                void op.run(
                  () => api('discardHeld', { id: discard.id }),
                  () => {
                    setDiscard(null);
                    list.reload();
                    toast.success('Venta descartada.');
                  },
                )
              }
            >
              Descartar venta
            </Button>
          </div>
        </Modal>
      )}
    </Modal>
  );
}
