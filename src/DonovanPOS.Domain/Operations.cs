namespace DonovanPOS.Domain;

public sealed class Sale : Entity
{
    public string Folio { get; set; } = "";
    public string RequestId { get; set; } = "";
    public DateTime CreatedAt { get; set; }
    public int UserId { get; set; }
    public string Cashier { get; set; } = "";
    public int? CustomerId { get; set; }
    public string CustomerName { get; set; } = "Público en general";
    public int CashSessionId { get; set; }
    public long TotalCents { get; set; }
    public long TaxCents { get; set; }
    public long DiscountCents { get; set; }
    public long ChangeCents { get; set; }
    public long TenderedCents { get; set; }
    public bool Cancelled { get; set; }
}
public sealed class SaleLine : Entity
{
    public int SaleId { get; set; }
    public int ProductId { get; set; }
    public string Sku { get; set; } = "";
    public string Name { get; set; } = "";
    public string Category { get; set; } = "";
    public Unit Unit { get; set; }
    public long QuantityUnits { get; set; }
    public long PriceCents { get; set; }
    public long? CostCents { get; set; }
    public int TaxBasisPoints { get; set; }
    public long TotalCents { get; set; }
    public long TaxCents { get; set; }
    public long ReturnedUnits { get; set; }
    public long RefundedCents { get; set; }
}
public sealed class PaymentRecord : Entity
{
    public int SaleId { get; set; }
    public PayMethod Method { get; set; }
    public long AmountCents { get; set; }
    public string Reference { get; set; } = "";
}
public sealed class StockMovement : Entity
{
    public int ProductId { get; set; }
    public DateTime CreatedAt { get; set; }
    public int UserId { get; set; }
    public string Kind { get; set; } = "";
    public string Reference { get; set; } = "";
    public string Reason { get; set; } = "";
    public long DeltaUnits { get; set; }
    public long BalanceUnits { get; set; }
}
public sealed class Purchase : Entity
{
    public int SupplierId { get; set; }
    public string Folio { get; set; } = "";
    public string SupplierReference { get; set; } = "";
    public DateTime CreatedAt { get; set; }
    public int UserId { get; set; }
    public long TotalCents { get; set; }
}
public sealed class PurchaseLine : Entity
{
    public int PurchaseId { get; set; }
    public int ProductId { get; set; }
    public long QuantityUnits { get; set; }
    public long CostCents { get; set; }
}
public sealed class CreditEntry : Entity
{
    public int CustomerId { get; set; }
    public int SaleId { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime? DueDate { get; set; }
    public long AmountCents { get; set; }
    public PayMethod Method { get; set; }
    public string Kind { get; set; } = "";
    public string Reference { get; set; } = "";
    public int UserId { get; set; }
}
public sealed class SaleReturn : Entity
{
    public int SaleId { get; set; }
    public DateTime CreatedAt { get; set; }
    public int UserId { get; set; }
    public string Folio { get; set; } = "";
    public string Reason { get; set; } = "";
    public long TotalCents { get; set; }
    public long CreditReductionCents { get; set; }
    public PayMethod RefundMethod { get; set; }
}
public sealed class ReturnLine : Entity
{
    public int SaleReturnId { get; set; }
    public int SaleLineId { get; set; }
    public long QuantityUnits { get; set; }
    public long AmountCents { get; set; }
}
public sealed class Quote : Entity
{
    public string Folio { get; set; } = "";
    public DateTime CreatedAt { get; set; }
    public DateTime ValidUntil { get; set; }
    public int? CustomerId { get; set; }
    public string CustomerName { get; set; } = "Público en general";
    public string Notes { get; set; } = "";
    public long TotalCents { get; set; }
    public long TaxCents { get; set; }
    public int? SaleId { get; set; }
    public int UserId { get; set; }
}
public sealed class QuoteLine : Entity
{
    public int QuoteId { get; set; }
    public int ProductId { get; set; }
    public string Name { get; set; } = "";
    public Unit Unit { get; set; }
    public long QuantityUnits { get; set; }
    public long PriceCents { get; set; }
    public int TaxBasisPoints { get; set; }
    public long TotalCents { get; set; }
}
