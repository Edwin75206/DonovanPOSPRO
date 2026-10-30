using DonovanPOS.Application;
using DonovanPOS.Domain;
using DonovanPOS.Presentation.Common;
namespace DonovanPOS.Presentation.ViewModels;

public sealed class CustomersVm : RecordsVm
{
    private readonly CustomersService customers; private readonly Dialogs dialogs; private readonly SalesService sales;
    public CustomersVm(CustomersService customers, SalesService sales, Dialogs dialogs, UiState ui) : base(ui)
    {
        this.customers = customers; this.dialogs = dialogs; this.sales = sales;
        Actions.Add(Action("Agregar cliente", () => Edit(null), true));
        RowActions.Add(Action("Editar cliente", () => Edit((Customer)Selected!.Source!)));
        RowActions.Add(Action("Registrar abono", Pay, true)); RowActions.Add(Action("Estado de cuenta", Ledger)); RowActions.Add(Action("Historial de compras", History));
    }
    public override string Title => "Clientes y créditos";
    public override string Subtitle => "Conoce a tus clientes y da seguimiento a cada saldo.";
    public override string[] Headers => ["Cliente", "Teléfono", "Límite de crédito", "Saldo pendiente", "Estado"];
    protected override async Task Reload()
    {
        var rows = await customers.Balances(); Summary = Mx.Money(rows.Sum(x => x.BalanceCents)) + " pendientes de cobro · " + rows.Count(x => x.BalanceCents > 0) + " clientes con saldo";
        SetRows(rows.Where(x => x.Customer.Name.Contains(Search, StringComparison.CurrentCultureIgnoreCase)).Select(x => new RecordRow(x.Customer.Id, x.Customer.Name, x.Customer.Phone, Mx.Money(x.Customer.CreditLimitCents), Mx.Money(x.BalanceCents), x.BalanceCents > 0 ? "Saldo pendiente" : "Al corriente", x.Customer)));
    }
    private async Task Edit(Customer? c)
    {
        c ??= new Customer(); var current = c;
        if (await dialogs.Form(c.Id == 0 ? "Agregar cliente" : "Editar cliente", "El límite de crédito se valida antes de cada venta.", [new("name", "Nombre", c.Name), new("phone", "Teléfono", c.Phone), new("email", "Correo", c.Email), new("address", "Dirección", c.Address), new("limit", "Límite de crédito MXN", Amount.Pesos(c.CreditLimitCents).ToString("0.00", Mx.Culture))], async f => { await customers.Save(new Customer { Id = current.Id, Name = f["name"], Phone = f["phone"], Email = f["email"], Address = f["address"], CreditLimitCents = Amount.Cents(f.Number("limit")) }); })) await Reload();
    }
    private async Task Pay()
    {
        var id = SelectedId(); var ledger = await customers.Ledger(id); var debts = ledger.GroupBy(x => x.SaleId).Where(x => x.Sum(e => e.AmountCents) > 0).Select(x => new Choice(x.Key.ToString(), $"{x.First(e => e.AmountCents > 0).Reference} · pendiente {Mx.Money(x.Sum(e => e.AmountCents))}")).ToArray();
        Amount.Require(debts.Length > 0, "Este cliente no tiene saldos pendientes.");
        if (await dialogs.Form("Registrar abono", Selected!.A + ". El abono se aplica a la venta seleccionada.", [new("sale", "Venta pendiente", debts[0].Value, debts), new("amount", "Importe del abono"), new("method", "Método", "Efectivo", Dialogs.Options<PayMethod>().Where(x => x.Value != "Credito").ToArray()), new("reference", "Referencia / observaciones")], async f => { await customers.Pay(id, f.Integer("sale"), f.Number("amount"), Enum.Parse<PayMethod>(f["method"]), f["reference"]); }, "Registrar abono")) await Reload();
    }
    private async Task Ledger()
    {
        var entries = await customers.Ledger(SelectedId());
        dialogs.Table(new("Estado de cuenta · " + Selected!.A, ["Fecha", "Concepto", "Referencia", "Importe MXN", "Vencimiento", "Método"], entries.Select(x => new[] { x.CreatedAt.ToString("dd/MM/yyyy HH:mm"), x.Kind, x.Reference, Mx.Money(x.AmountCents), x.DueDate?.ToString("dd/MM/yyyy") ?? "", x.Method.ToString() }).ToList()));
    }
    private async Task History()
    {
        var customerId = SelectedId(); var ledger = await customers.Ledger(customerId); var purchases = await customers.Purchases(customerId);
        var rows = purchases.Select(s => new[] { s.Folio, s.CreatedAt.ToString("dd/MM/yyyy"), Mx.Money(s.TotalCents), Mx.Money(ledger.Where(x => x.SaleId == s.Id).Sum(x => x.AmountCents)) }).ToList();
        dialogs.Table(new("Historial de compras · " + Selected!.A, ["Folio", "Fecha", "Importe", "Saldo"], rows));
    }
}
