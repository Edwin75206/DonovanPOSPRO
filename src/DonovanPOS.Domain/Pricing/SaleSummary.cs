// Adapted from SupermarketPos, Copyright (c) 2026 Sm3th, MIT. See licenses/SupermarketPos.txt.
namespace DonovanPOS.Domain.Pricing;

/// <summary>Final charged gross for a single cart line after line- and cart-level discounts.</summary>
public sealed record LineCharge(CartLine Line, decimal FinalGross);

/// <summary>
/// Fully-resolved money picture of a cart: the per-line final charges, the VAT breakdown,
/// and the headline totals. Produced by <see cref="PricingService.Summarize"/>.
/// </summary>
public sealed record SaleSummary(
    IReadOnlyList<LineCharge> LineCharges,
    IReadOnlyList<VatGroup> VatBreakdown,
    decimal GrossBeforeCartDiscount,
    decimal GrossTotal,
    decimal NetTotal,
    decimal VatTotal);

