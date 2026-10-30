// Adapted from SupermarketPos, Copyright (c) 2026 Sm3th, MIT. See licenses/SupermarketPos.txt.
namespace DonovanPOS.Domain.Pricing;

/// <summary>
/// VAT (IVA) totals for a single rate. Gross is IVA-inclusive; Net is the pre-tax base;
/// Vat is the tax portion (Gross - Net). All values are already rounded to 2 decimals.
/// </summary>
public sealed record VatGroup(decimal Rate, decimal Gross, decimal Net, decimal Vat);

