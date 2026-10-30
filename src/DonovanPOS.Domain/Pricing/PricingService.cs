// Adapted from SupermarketPos, Copyright (c) 2026 Sm3th, MIT. See licenses/SupermarketPos.txt.
using DonovanPOS.Domain;

namespace DonovanPOS.Domain.Pricing;

/// <summary>
/// THE money core. Every total, VAT extraction, and discount distribution in the app routes
/// through here so the arithmetic exists in exactly one place.
///
/// Rules (Tax-inclusive retail):
///  - Shelf prices are IVA-inclusive (gross).
///  - VAT is EXTRACTED from the inclusive gross, grouped by rate, to avoid rounding drift.
///  - Everything rounds to 2 decimals AwayFromZero (<see cref="Amount.Round"/>).
///  - A whole-cart discount is distributed PROPORTIONALLY across lines before VAT extraction,
///    so each VAT group is taxed on its discounted share.
/// </summary>
public static class PricingService
{
    /// <summary>
    /// Per-rate VAT breakdown for a set of lines (no cart-level discount applied).
    /// Group by rate, sum the gross, then extract net &amp; vat once per group.
    /// </summary>
    public static IReadOnlyList<VatGroup> VatBreakdown(IEnumerable<CartLine> lines) =>
        ExtractGroups(lines.Select(l => (l.VatRate, l.GrossTotal)));

    /// <summary>Gross total of the cart including any whole-cart discount.</summary>
    public static decimal GrossTotal(Cart cart)
    {
        var grossBefore = Amount.Round(cart.Lines.Sum(l => l.GrossTotal));
        return cart.CartDiscount?.Apply(grossBefore) ?? grossBefore;
    }

    /// <summary>
    /// Resolve the cart into final per-line charges, a VAT breakdown, and totals.
    /// The cart discount is spread across lines in proportion to their gross, and the
    /// rounding residual is parked on the largest line so the parts sum to the whole exactly.
    /// </summary>
    public static SaleSummary Summarize(Cart cart)
    {
        var grossBefore = Amount.Round(cart.Lines.Sum(l => l.GrossTotal));
        var grossAfter = cart.CartDiscount?.Apply(grossBefore) ?? grossBefore;

        var allocated = Amount.Allocate(Amount.Cents(grossAfter), cart.Lines.Select(l => Amount.Cents(l.GrossTotal)).ToArray());
        var charges = cart.Lines.Select((line, i) => new LineCharge(line, Amount.Pesos(allocated[i]))).ToList();

        var breakdown = ExtractGroups(charges.Select(c => (c.Line.VatRate, c.FinalGross)));
        var net = Amount.Round(breakdown.Sum(g => g.Net));
        var vat = Amount.Round(breakdown.Sum(g => g.Vat));

        return new SaleSummary(charges, breakdown, grossBefore, grossAfter, net, vat);
    }

    /// <summary>Extract per-rate VAT groups from raw (rate, inclusive-gross) pairs.</summary>
    public static IReadOnlyList<VatGroup> ExtractGroups(IEnumerable<(decimal Rate, decimal Gross)> items) =>
        items
            .GroupBy(x => x.Rate)
            .Select(g =>
            {
                var gross = Amount.Round(g.Sum(x => x.Gross));
                var netBase = Amount.Round(gross / (1 + g.Key));
                var vat = Amount.Round(gross - netBase);
                return new VatGroup(g.Key, gross, netBase, vat);
            })
            .OrderBy(v => v.Rate)
            .ToList();
}

