// Adapted from SupermarketPos, Copyright (c) 2026 Sm3th, MIT. See licenses/SupermarketPos.txt.
using DonovanPOS.Domain;

namespace DonovanPOS.Domain.Pricing;

/// <summary>
/// A line in an in-progress sale: a product, a quantity, and an optional per-line discount.
/// All amounts are IVA-inclusive (gross), matching the shelf price.
/// </summary>
public sealed class CartLine
{
    public Product Product { get; }
    public decimal Quantity { get; private set; }
    public Discount? LineDiscount { get; private set; }

    public CartLine(Product product, decimal quantity)
    {
        Product = product ?? throw new RuleException("Selecciona un producto.");
        SetQuantity(quantity);
    }

    public decimal GrossUnitPrice => Product.UnitPrice;                           // IVA-inclusive
    public decimal GrossSubtotal => Amount.Round(GrossUnitPrice * Quantity);      // before line discount
    public decimal GrossTotal => LineDiscount?.Apply(GrossSubtotal) ?? GrossSubtotal;
    public decimal VatRate => Product.VatRate;

    public void SetQuantity(decimal quantity)
    {
        if (quantity <= 0) throw new RuleException("La cantidad debe ser mayor que cero.");
        Amount.QuantityFor(Product.Unit, Amount.Units(quantity));
        Quantity = quantity;
    }

    public void Increment(decimal by)
    {
        if (by <= 0) throw new RuleException("La cantidad debe ser positiva.");
        SetQuantity(Quantity + by);
    }

    public void ApplyDiscount(Discount? discount) => LineDiscount = discount;
}


