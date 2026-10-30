#if DEBUG
using DonovanPOS.Application;
using DonovanPOS.Domain;
using Microsoft.Extensions.DependencyInjection;
namespace DonovanPOS.Presentation.QualityAssurance;

internal sealed class QaClock : IClock
{
    public DateTime Now { get; set; } = new DonovanPOS.Infrastructure.MexicoClock().Now;
}
internal static class QaData
{
    public static async Task Prepare(ServiceProvider services)
    {
        T Get<T>() where T : notnull => services.GetRequiredService<T>();
        var password = Guid.NewGuid().ToString("N"); await Get<AuthService>().Setup("Ferretería La Central · DEMOSTRACIÓN", "qa", "Edwin · pruebas", password); await Get<AuthService>().Login("qa", password);
        var clock = (QaClock)Get<IClock>(); var now = clock.Now;
        (string sku, string name, string category, string brand, Unit unit, decimal price, decimal stock)[] products =
        [
            ("HER-001", "Martillo de uña 16 oz", "Herramientas", "Truper", Unit.Pieza, 189, 24),
            ("HER-002", "Taladro percutor 1/2 pulgada", "Herramientas", "Makita", Unit.Pieza, 1499, 7),
            ("HER-003", "Juego de desarmadores, 6 piezas", "Herramientas", "Truper", Unit.Pieza, 245, 18),
            ("ELE-001", "Cable THW calibre 12", "Eléctrico", "Condumex", Unit.Metro, 18.5m, 450),
            ("ELE-002", "Contacto dúplex polarizado", "Eléctrico", "Volteck", Unit.Pieza, 42, 60),
            ("PLO-001", "Tubo PVC hidráulico 1/2 pulgada", "Plomería", "Durman", Unit.Pieza, 65, 35),
            ("PLO-002", "Cinta teflón 1/2 pulgada", "Plomería", "Foset", Unit.Pieza, 12, 90),
            ("TOR-001", "Tornillo para madera 1 pulgada", "Tornillería", "Fiero", Unit.Kilogramo, 89, 30),
            ("PIN-001", "Pintura vinílica blanco mate", "Pinturas", "Comex", Unit.Litro, 159, 80),
            ("PIN-002", "Brocha profesional 2 pulgadas", "Pinturas", "Pretul", Unit.Pieza, 48, 40),
            ("SEG-001", "Guantes de trabajo reforzados", "Seguridad", "Truper", Unit.Pieza, 79, 3),
            ("HER-004", "Disco de corte metal 4 1/2", "Herramientas", "Austromex", Unit.Pieza, 29, 4),
            ("PLO-003", "Llave de jardín 1/2 pulgada", "Plomería", "Foset", Unit.Pieza, 115, 2),
            ("ELE-003", "Cinta aislante negra 18 m", "Eléctrico", "3M", Unit.Pieza, 39, 5)
        ];
        List<(int id, decimal price)> ids = [];
        foreach (var p in products) ids.Add((await Get<CatalogService>().Save(new Product { Sku = p.sku, Barcode = "750100" + (ids.Count + 1).ToString("0000"), Name = p.name, Category = p.category, Brand = p.brand, Unit = p.unit, PriceCents = Amount.Cents(p.price), WholesaleCents = Amount.Cents(p.price * .9m), CostCents = Amount.Cents(p.price * .56m), StockUnits = Amount.Units(p.stock), MinimumUnits = 50000, PackName = p.unit == Unit.Metro ? "Rollo" : "Caja", PackUnits = p.unit == Unit.Metro ? 1000000 : 10000 }), p.price));
        var customer = await Get<CustomersService>().Save(new Customer { Name = "Construcciones del Centro (prueba)", Phone = "555 000 0000", CreditLimitCents = 2000000 });
        await Get<CustomersService>().Save(new Customer { Name = "Pinturas y acabados Morales (prueba)", CreditLimitCents = 500000 });
        await Get<PurchasesService>().SaveSupplier(new Supplier { Name = "Distribuidora de herramientas (prueba)", Phone = "555 000 0001" });
        for (var day = 6; day >= 0; day--)
        {
            clock.Now = now.Date.AddDays(-day).AddHours(9); await Get<CashService>().Open(1000);
            for (var j = 0; j < 4; j++)
            {
                clock.Now = now.Date.AddDays(-day).AddHours(9 + j); var p = ids[(day * 3 + j) % 10]; var q = p.id == ids[1].id ? 1 : 2; var total = p.price * q; var method = j % 3 == 0 ? PayMethod.Efectivo : j % 3 == 1 ? PayMethod.Tarjeta : PayMethod.Transferencia;
                await Get<SalesService>().Checkout(new(Guid.NewGuid().ToString(), [new(p.id, q)], [new(method, total)], method == PayMethod.Efectivo ? total : 0));
            }
            if (day != 0) await Get<CashService>().Close(Amount.Pesos((await Get<CashService>().Overview()).ExpectedCents), "Cierre de demostración");
        }
        clock.Now = now; var creditProduct = ids[2]; await Get<SalesService>().Checkout(new(Guid.NewGuid().ToString(), [new(creditProduct.id, 2)], [new(PayMethod.Credito, creditProduct.price * 2)], 0, CustomerId: customer, DueDate: now.AddDays(15)));
        await Get<QuotesService>().Save(0, [new(ids[0].id, 2), new(ids[3].id, 20)], customer, 0, now.AddDays(15), "Datos de demostración, sin validez comercial.");
    }
}
#endif
