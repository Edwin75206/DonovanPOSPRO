use crate::error::{Result, ensure};

// Integer cents and quantities scaled by 10,000; i128 intermediates prevent overflow.
pub const MAX_AMOUNT: i64 = 100_000_000_000;
pub const MAX_UNITS: i64 = 1_000_000_000_000;
pub fn rounded(numerator: i128, denominator: i128) -> i64 {
    let sign = if numerator < 0 { -1 } else { 1 };
    (sign * ((numerator.abs() + denominator / 2) / denominator)) as i64
}
pub fn line_total(cents: i64, units: i64) -> Result<i64> {
    ensure(
        (0..=MAX_AMOUNT).contains(&cents) && (0..=MAX_UNITS).contains(&units),
        "VALIDATION",
        "El importe o la cantidad supera el límite permitido.",
    )?;
    let result = (cents as i128 * units as i128 + 5000) / 10_000;
    ensure(
        result <= MAX_AMOUNT as i128,
        "VALIDATION",
        "El importe de la partida supera el límite permitido.",
    )?;
    Ok(result as i64)
}
pub fn quantity(unit: i64, units: i64) -> Result<()> {
    ensure(
        (0..=5).contains(&unit) && units > 0 && units <= MAX_UNITS,
        "VALIDATION",
        "Indica una cantidad mayor que cero y una unidad válida.",
    )?;
    ensure(
        ![0, 4, 5].contains(&unit) || units % 10_000 == 0,
        "VALIDATION",
        "Las piezas, cajas y rollos requieren cantidades enteras.",
    )
}
pub fn allocate(total: i64, weights: &[i64]) -> Result<Vec<i64>> {
    ensure(
        total >= 0 && weights.iter().all(|&x| x >= 0),
        "VALIDATION",
        "No se pueden distribuir importes negativos.",
    )?;
    let sum: i128 = weights.iter().map(|&x| x as i128).sum();
    if sum == 0 {
        ensure(
            total == 0,
            "VALIDATION",
            "No hay partidas para distribuir el importe.",
        )?;
        return Ok(vec![0; weights.len()]);
    }
    let raw: Vec<i128> = weights.iter().map(|&x| x as i128 * total as i128).collect();
    let mut parts: Vec<i64> = raw.iter().map(|x| (x / sum) as i64).collect();
    let remaining = (total - parts.iter().sum::<i64>()) as usize;
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(raw[i] % sum), i));
    for &i in order.iter().take(remaining) {
        parts[i] += 1;
    }
    Ok(parts)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pennies_and_fractional_quantities() {
        assert_eq!(allocate(1, &[1, 1, 1]).unwrap(), [1, 0, 0]);
        assert_eq!(allocate(100, &[1, 1, 1]).unwrap(), [34, 33, 33]);
        assert_eq!(line_total(1999, 3333).unwrap(), 666);
        assert_eq!(rounded(-15, 10), -2);
        assert!(quantity(0, 5000).is_err());
        assert!(quantity(1, 5000).is_ok());
    }
}
