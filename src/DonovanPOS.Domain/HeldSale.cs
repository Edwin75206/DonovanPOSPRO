namespace DonovanPOS.Domain;
public sealed class HeldSale : Entity
{
    public string Folio { get; set; } = "";
    public string Label { get; set; } = "";
    public DateTime CreatedAt { get; set; }
    public int UserId { get; set; }
    public int? CustomerId { get; set; }
    public int? SaleId { get; set; }
    public int DiscountParts { get; set; }
    public bool Discarded { get; set; }
}
public sealed class HeldSaleLine : Entity
{
    public int HeldSaleId { get; set; }
    public int ProductId { get; set; }
    public long QuantityUnits { get; set; }
    public bool Wholesale { get; set; }
    public long OriginalPriceCents { get; set; }
}
