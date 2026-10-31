import { useState } from 'react';
import { Plus, Search, Pencil, Wallet, Users, Truck } from 'lucide-react';
import { toast } from 'sonner';
import { api, can, type Actor, type Contact, type CustomerLedger } from '@/lib/api';
import { useResource } from '@/lib/useResource';
import { dateTime, money, parseScaled, today } from '@/lib/utils';
import { DataGrid } from '@/components/DataGrid';
import { Button, Busy, Failure, Field, Modal, PageTitle, useOperation } from '@/components/common';

export const newContact = (): Contact => ({
  id: 0,
  name: '',
  phone: '',
  email: '',
  address: '',
  creditLimitCents: 0,
  active: true,
});
export function Contacts({ actor, supplier = false }: { actor: Actor; supplier?: boolean }) {
  const [search, setSearch] = useState('');
  const [archived, setArchived] = useState(false);
  const [editing, setEditing] = useState<Contact | null>(null);
  const [ledgerId, setLedgerId] = useState<number | null>(null);
  const editable = can(actor, supplier ? 'purchase' : 'credit');
  const list = useResource(
    () => api('contacts', { supplier, search, archived }),
    [supplier, search, archived],
  );
  return (
    <main className="page-scroll page-container records-page">
      <PageTitle
        eyebrow={supplier ? 'ABASTECIMIENTO' : 'RELACIONES QUE CRECEN'}
        title={supplier ? 'Tus proveedores, a la mano.' : 'Conoce mejor a tus clientes.'}
        description={
          supplier
            ? 'Contacto, compras e historial de tus aliados.'
            : 'Encuentra sus datos y da seguimiento a sus cuentas.'
        }
        actions={
          editable && (
            <Button onClick={() => setEditing(newContact())}>
              <Plus size={17} /> {supplier ? 'Nuevo proveedor' : 'Nuevo cliente'}
            </Button>
          )
        }
      />
      <div className="records-toolbar">
        <div className="records-search">
          <Search size={18} />
          <input
            aria-label={supplier ? 'Buscar proveedor' : 'Buscar cliente'}
            placeholder="Busca por nombre o teléfono…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            maxLength={300}
          />
        </div>
        <label className="check-label">
          <input
            type="checkbox"
            checked={archived}
            onChange={(e) => setArchived(e.target.checked)}
          />{' '}
          Ver archivados
        </label>
        <span>
          {list.busy
            ? 'Buscando…'
            : `${list.data?.length ?? 0} contactos${list.data?.length === 1000 ? ' · acota la búsqueda para ver más' : ''}`}
        </span>
      </div>
      <Failure error={list.error} retry={list.reload} />
      <section className="surface" aria-busy={list.busy}>
        <DataGrid
          rows={list.data ?? []}
          caption={supplier ? 'Proveedores' : 'Clientes'}
          emptyTitle={list.busy ? 'Buscando contactos…' : 'Aquí empieza una buena relación.'}
          columns={[
            {
              id: 'name',
              label: 'Nombre',
              value: (c) => c.name,
              render: (c) => (
                <div className="record-name">
                  <span className="record-avatar">
                    {supplier ? <Truck size={18} /> : <Users size={18} />}
                  </span>
                  <div>
                    <strong>{c.name}</strong>
                    <small>{c.email || c.address || 'Sin datos adicionales'}</small>
                  </div>
                </div>
              ),
            },
            { id: 'phone', label: 'Teléfono', value: (c) => c.phone || '—' },
            ...(!supplier && editable
              ? [
                  {
                    id: 'balance',
                    label: 'Por cobrar',
                    value: (c: Contact) => c.balanceCents ?? 0,
                    render: (c: Contact) => (
                      <b className={c.balanceCents ? 'amount-pending' : ''}>
                        {money(c.balanceCents)}
                      </b>
                    ),
                    align: 'right' as const,
                  },
                ]
              : []),
            {
              id: 'actions',
              label: 'Acciones',
              value: (c) => c.id,
              sortable: false,
              render: (c) => (
                <div className="row-actions">
                  {!supplier && editable && (
                    <Button variant="outline" size="sm" onClick={() => setLedgerId(c.id)}>
                      <Wallet size={15} /> Cuenta
                    </Button>
                  )}
                  {editable && (
                    <Button
                      variant="ghost"
                      size="sm"
                      aria-label={`Editar ${c.name}`}
                      onClick={() => setEditing(c)}
                    >
                      <Pencil size={15} /> Editar
                    </Button>
                  )}
                </div>
              ),
            },
          ]}
        />
      </section>
      {editing && (
        <ContactEditor
          key={`${supplier}-${editing.id}`}
          contact={editing}
          supplier={supplier}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            list.reload();
          }}
        />
      )}
      {ledgerId && (
        <Ledger
          key={ledgerId}
          id={ledgerId}
          onClose={() => {
            setLedgerId(null);
            list.reload();
          }}
        />
      )}
    </main>
  );
}
export function ContactEditor({
  contact,
  supplier = false,
  onClose,
  onSaved,
}: {
  contact: Contact;
  supplier?: boolean;
  onClose: () => void;
  onSaved: (contact: Contact) => void;
}) {
  const [draft, setDraft] = useState({ ...newContact(), ...contact });
  const op = useOperation();
  return (
    <Modal
      open
      onClose={() => {
        if (!op.busy) onClose();
      }}
      title={`${contact.id ? 'Editar' : 'Nuevo'} ${supplier ? 'proveedor' : 'cliente'}`}
      description="Completa lo necesario. Podrás actualizarlo después."
    >
      <form
        className="record-form"
        onSubmit={(e) => {
          e.preventDefault();
          void op.run(
            () => api('saveContact', { supplier, contact: draft }),
            (result) => {
              toast.success('Contacto guardado.');
              onSaved(result);
            },
          );
        }}
      >
        <fieldset disabled={op.busy}>
          <Field label="Nombre">
            <input
              autoFocus
              required
              maxLength={150}
              value={draft.name}
              onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            />
          </Field>
          <div className="form-grid">
            <Field label="Teléfono">
              <input
                type="tel"
                maxLength={60}
                value={draft.phone}
                onChange={(e) => setDraft({ ...draft, phone: e.target.value })}
              />
            </Field>
            <Field label="Correo">
              <input
                type="email"
                maxLength={200}
                value={draft.email}
                onChange={(e) => setDraft({ ...draft, email: e.target.value })}
              />
            </Field>
          </div>
          <Field label="Dirección">
            <textarea
              maxLength={500}
              value={draft.address}
              onChange={(e) => setDraft({ ...draft, address: e.target.value })}
            />
          </Field>
          {!supplier && (
            <Field
              label="Límite de crédito (MXN)"
              hint="Con $0 no se podrán registrar nuevas ventas a crédito."
            >
              <input
                type="number"
                min="0"
                step="0.01"
                max="1000000000"
                value={draft.creditLimitCents / 100}
                onChange={(e) =>
                  setDraft({ ...draft, creditLimitCents: parseScaled(e.target.value) })
                }
              />
            </Field>
          )}
          {!!contact.id && (
            <label className="check-label">
              <input
                type="checkbox"
                checked={!draft.active}
                onChange={(e) => setDraft({ ...draft, active: !e.target.checked })}
              />{' '}
              Archivar contacto (conserva su historial)
            </label>
          )}
        </fieldset>
        <Failure error={op.error} />
        <div className="dialog-actions">
          <Button type="button" variant="outline" disabled={op.busy} onClick={onClose}>
            Cancelar
          </Button>
          <Button type="submit" disabled={op.busy}>
            {op.busy ? 'Guardando…' : 'Guardar contacto'}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
function Ledger({ id, onClose }: { id: number; onClose: () => void }) {
  const ledger = useResource(() => api('customerLedger', { id }), [id]);
  const [payment, setPayment] = useState(false);
  return (
    <Modal
      open
      onClose={onClose}
      title={ledger.data?.customer.name ?? 'Cuenta del cliente'}
      description="Ventas a crédito, abonos y devoluciones en un solo lugar."
      className="records-dialog"
    >
      <Failure error={ledger.error} retry={ledger.reload} />
      {!ledger.data ? (
        <Busy />
      ) : (
        <>
          <div className="account-summary">
            <div>
              <span>SALDO PENDIENTE</span>
              <strong>{money(ledger.data.balanceCents)}</strong>
              <small>Límite autorizado: {money(ledger.data.customer.creditLimitCents)}</small>
            </div>
            <Button disabled={ledger.data.balanceCents <= 0} onClick={() => setPayment(true)}>
              <Plus size={17} /> Registrar abono
            </Button>
          </div>
          <h3>Ventas pendientes</h3>
          <DataGrid
            rows={ledger.data.sales.filter((s) => s.balanceCents > 0)}
            caption="Ventas a crédito pendientes"
            emptyTitle="Este cliente está al corriente."
            columns={[
              { id: 'folio', label: 'Venta', value: (s) => s.folio },
              {
                id: 'date',
                label: 'Vencimiento',
                value: (s) => s.dueDate,
                render: (s) => (
                  <span className={s.dueDate && s.dueDate < today() ? 'amount-pending' : ''}>
                    {s.dueDate || 'Sin fecha'}
                  </span>
                ),
              },
              {
                id: 'total',
                label: 'Venta total',
                value: (s) => s.totalCents,
                render: (s) => money(s.totalCents),
                align: 'right',
              },
              {
                id: 'balance',
                label: 'Pendiente',
                value: (s) => s.balanceCents,
                render: (s) => <b>{money(s.balanceCents)}</b>,
                align: 'right',
              },
            ]}
          />
          <h3>Movimientos de la cuenta</h3>
          <DataGrid
            rows={ledger.data.entries}
            caption="Movimientos de crédito"
            columns={[
              {
                id: 'date',
                label: 'Fecha',
                value: (e) => e.createdAt,
                render: (e) => dateTime(e.createdAt),
              },
              { id: 'sale', label: 'Venta', value: (e) => e.saleFolio },
              { id: 'kind', label: 'Movimiento', value: (e) => e.kind },
              { id: 'reference', label: 'Referencia', value: (e) => e.reference },
              {
                id: 'amount',
                label: 'Importe',
                value: (e) => e.amountCents,
                render: (e) => (
                  <b className={e.amountCents < 0 ? 'amount-positive' : ''}>
                    {money(e.amountCents)}
                  </b>
                ),
                align: 'right',
              },
            ]}
          />
          {payment && (
            <CreditPaymentForm
              ledger={ledger.data}
              onClose={() => setPayment(false)}
              onSaved={() => {
                setPayment(false);
                ledger.reload();
              }}
            />
          )}
        </>
      )}
    </Modal>
  );
}
function CreditPaymentForm({
  ledger,
  onClose,
  onSaved,
}: {
  ledger: CustomerLedger;
  onClose: () => void;
  onSaved: () => void;
}) {
  const pending = ledger.sales.filter((s) => s.balanceCents > 0);
  const [saleId, setSaleId] = useState(pending[0]?.id ?? 0);
  const [amount, setAmount] = useState('');
  const [method, setMethod] = useState(0);
  const [reference, setReference] = useState('');
  const [requestId] = useState(() => crypto.randomUUID());
  const [confirmed, setConfirmed] = useState(false);
  const op = useOperation();
  const balance = pending.find((s) => s.id === saleId)?.balanceCents ?? 0;
  return (
    <Modal
      open
      onClose={() => {
        if (!op.busy) onClose();
      }}
      title="Registrar abono"
      description="El abono se aplicará únicamente a la venta seleccionada."
    >
      <form
        className="record-form"
        onSubmit={(e) => {
          e.preventDefault();
          if (!confirmed) return;
          void op.run(
            () =>
              api('creditPayment', {
                requestId,
                customerId: ledger.customer.id,
                saleId,
                amountCents: parseScaled(amount),
                method,
                reference,
              }),
            () => {
              toast.success('Abono registrado.');
              onSaved();
            },
          );
        }}
      >
        <fieldset disabled={op.busy}>
          <Field label="Venta pendiente">
            <select
              value={saleId}
              onChange={(e) => {
                setSaleId(Number(e.target.value));
                setConfirmed(false);
              }}
            >
              {pending.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.folio} · {money(s.balanceCents)}
                </option>
              ))}
            </select>
          </Field>
          <div className="form-grid">
            <Field label="Importe recibido (MXN)">
              <input
                autoFocus
                type="number"
                min="0.01"
                max={balance / 100}
                step="0.01"
                required
                value={amount}
                onChange={(e) => {
                  setAmount(e.target.value);
                  setConfirmed(false);
                }}
              />
            </Field>
            <Field label="Forma de pago">
              <select value={method} onChange={(e) => setMethod(Number(e.target.value))}>
                {['Efectivo', 'Tarjeta', 'Transferencia'].map((m, i) => (
                  <option key={m} value={i}>
                    {m}
                  </option>
                ))}
              </select>
            </Field>
          </div>
          <Field label="Referencia (opcional)">
            <input
              maxLength={200}
              value={reference}
              onChange={(e) => setReference(e.target.value)}
            />
          </Field>
          <label className="check-label">
            <input
              type="checkbox"
              required
              checked={confirmed}
              onChange={(e) => setConfirmed(e.target.checked)}
            />{' '}
            Confirmo que recibí {money(parseScaled(amount))}.
          </label>
          <p className="muted">
            La caja debe estar abierta. Solo el efectivo modifica el dinero esperado en caja.
          </p>
        </fieldset>
        <Failure error={op.error} />
        <Button className="wide" type="submit" disabled={op.busy || !confirmed}>
          {op.busy ? 'Registrando…' : 'Confirmar abono'}
        </Button>
      </form>
    </Modal>
  );
}
