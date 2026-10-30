namespace DonovanPOS.Domain;

public sealed class RuleException(string message) : Exception(message);
public abstract class Entity { public int Id { get; set; } }
public enum Role { Administrador, Cajero }
public enum Unit { Pieza, Metro, Kilogramo, Litro, Caja, Rollo }
public enum PayMethod { Efectivo, Tarjeta, Transferencia, Credito }
public enum Permission { Sell, Catalog, Stock, Purchase, Credit, Quote, Reports, Cash, Users, Settings, Discount, Return }

public static class Amount
{
    public static decimal Round(decimal value) => Math.Round(value, 2, MidpointRounding.AwayFromZero);
    public static long Cents(decimal value) => checked((long)(Round(value) * 100));
    public static decimal Pesos(long cents) => cents / 100m;
    public static long Units(decimal value)
    {
        Require(value == decimal.Round(value, 4), "Usa como máximo 4 decimales para la cantidad.");
        return checked((long)(value * 10000));
    }
    public static decimal Quantity(long units) => units / 10000m;
    /// <summary>Largest-remainder allocation in cents. Never creates negative lines; all parts sum exactly.</summary>
    public static long[] Allocate(long total, IReadOnlyList<long> weights)
    {
        Require(total >= 0 && weights.All(x => x >= 0), "No se pueden distribuir importes negativos.");
        var sum = weights.Sum();
        if (sum == 0) { Require(total == 0, "No hay partidas para distribuir el importe."); return new long[weights.Count]; }
        var raw = weights.Select(x => (decimal)x * total / sum).ToArray();
        var result = raw.Select(x => checked((long)decimal.Floor(x))).ToArray();
        var remainder = checked((int)(total - result.Sum()));
        foreach (var i in Enumerable.Range(0, result.Length).OrderByDescending(i => raw[i] - result[i]).ThenBy(i => i).Take(remainder)) result[i]++;
        return result;
    }
    public static void Require(bool condition, string message) { if (!condition) throw new RuleException(message); }
    public static string Text(string value, string label, int max = 200)
    {
        value = value.Trim();
        Require(value.Length > 0 && value.Length <= max, $"{label}: escribe entre 1 y {max} caracteres.");
        return value;
    }
    public static void QuantityFor(Unit unit, long quantity)
    {
        Require(quantity > 0, "La cantidad debe ser mayor que cero.");
        if (unit is Unit.Pieza or Unit.Caja or Unit.Rollo)
            Require(quantity % 10000 == 0, "Esta unidad se vende en cantidades enteras.");
    }
}
