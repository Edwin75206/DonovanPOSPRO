// Adapted from SupermarketPos, Copyright (c) 2026 Sm3th, MIT. See licenses/SupermarketPos.txt.
using DonovanPOS.Domain;

namespace DonovanPOS.Domain.Pricing;

/// <summary>
/// Discount strategy (polymorphism #1). A discount transforms a gross amount into a smaller,
/// never-negative gross amount. Concrete strategies decide how.
/// </summary>
public abstract class Discount
{
    public abstract decimal Apply(decimal gross);
    public abstract string Describe();
}

/// <summary>Percentage off, rate in the range 0..1 (e.g. 0.10 = 10% off).</summary>
public sealed class PercentageDiscount : Discount
{
    public decimal Rate { get; }

    public PercentageDiscount(decimal rate)
    {
        if (rate < 0 || rate > 1)
            throw new RuleException("El descuento debe estar entre 0 y 100%.");
        Rate = rate;
    }

    public override decimal Apply(decimal gross) => global::DonovanPOS.Domain.Amount.Round(gross * (1 - Rate));

    public override string Describe() => $"-%{Rate * 100:0.#}";
}

/// <summary>A fixed amount off the gross, clamped so the result never drops below zero.</summary>
public sealed class FixedAmountDiscount : Discount
{
    public decimal Amount { get; }

    public FixedAmountDiscount(decimal amount)
    {
        if (amount <= 0)
            throw new RuleException("El descuento debe ser mayor que cero.");
        Amount = global::DonovanPOS.Domain.Amount.Round(amount);
    }

    public override decimal Apply(decimal gross) => Math.Max(0m, global::DonovanPOS.Domain.Amount.Round(gross - Amount));

    public override string Describe() => $"-{Amount:0.00}{" MXN"}";
}


