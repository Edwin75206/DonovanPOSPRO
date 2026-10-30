using System.ComponentModel.DataAnnotations.Schema;
namespace DonovanPOS.Domain;

public sealed class Product : Entity
{
    public string Sku { get; set; } = "";
    public string? Barcode { get; set; }
    public string Name { get; set; } = "";
    public string Category { get; set; } = "General";
    public string Brand { get; set; } = "";
    public string Size { get; set; } = "";
    public Unit Unit { get; set; }
    public long? CostCents { get; set; }
    public long PriceCents { get; set; }
    public long WholesaleCents { get; set; }
    public long StockUnits { get; set; }
    public long MinimumUnits { get; set; }
    public int TaxBasisPoints { get; set; } = 1600;
    public string PackName { get; set; } = "Caja";
    public long PackUnits { get; set; } = 10000;
    public bool Active { get; set; } = true;
    [NotMapped] public decimal UnitPrice => Amount.Pesos(PriceCents);
    [NotMapped] public decimal VatRate => TaxBasisPoints / 10000m;
    [NotMapped] public decimal Stock => Amount.Quantity(StockUnits);
    [NotMapped] public decimal Minimum => Amount.Quantity(MinimumUnits);
    [NotMapped] public string StockLabel => $"{Stock:0.####} {Unit}";
    [NotMapped] public string StockStatus => !Active ? "Archivado" : StockUnits <= MinimumUnits ? "Pocas existencias" : "Disponible";
    public void Validate()
    {
        Sku = Amount.Text(Sku, "Clave", 40).ToUpperInvariant();
        Name = Amount.Text(Name, "Descripción");
        Category = Amount.Text(Category, "Categoría", 80);
        Barcode = string.IsNullOrWhiteSpace(Barcode) ? null : Amount.Text(Barcode, "Código", 64);
        Amount.Require(Enum.IsDefined(Unit), "Selecciona una unidad válida.");
        Amount.Require(PriceCents >= 0 && WholesaleCents >= 0 && (CostCents is null or >= 0), "Los precios y el costo no pueden ser negativos.");
        Amount.Require(StockUnits >= 0 && MinimumUnits >= 0 && PackUnits > 0, "Revisa las existencias, el mínimo y la conversión.");
        Amount.QuantityFor(Unit, PackUnits);
        if (StockUnits > 0) Amount.QuantityFor(Unit, StockUnits);
        if (MinimumUnits > 0) Amount.QuantityFor(Unit, MinimumUnits);
        Amount.Require(TaxBasisPoints is >= 0 and <= 10000, "El impuesto debe estar entre 0 y 100%.");
    }
    public void Move(long delta)
    {
        Amount.Require(checked(StockUnits + delta) >= 0, $"No hay suficientes existencias de {Name}. Disponibles: {StockLabel}.");
        StockUnits = checked(StockUnits + delta);
    }
}
public sealed class Customer : Entity
{
    public string Name { get; set; } = "";
    public string Phone { get; set; } = "";
    public string Email { get; set; } = "";
    public string Address { get; set; } = "";
    public long CreditLimitCents { get; set; }
    public bool Active { get; set; } = true;
    public override string ToString() => Name;
}
public sealed class Supplier : Entity
{
    public string Name { get; set; } = "";
    public string Phone { get; set; } = "";
    public string Email { get; set; } = "";
    public string Address { get; set; } = "";
    public bool Active { get; set; } = true;
    public override string ToString() => Name;
}
