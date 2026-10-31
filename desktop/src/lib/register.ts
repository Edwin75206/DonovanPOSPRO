import type { Product } from './api';
import { quantity, units } from './utils';

export function stockState(product: Product, inCart = 0) {
  const available = Math.max(0, product.stockUnits - inCart);
  const state = available === 0 ? 'empty' : available <= product.minimumUnits ? 'low' : 'available';
  return {
    available,
    state,
    label:
      state === 'empty'
        ? product.stockUnits > 0
          ? 'Todo en tu ticket'
          : 'Agotado'
        : state === 'low'
          ? `Últimas ${quantity(available)} ${units[product.unit]}`
          : `${quantity(available)} ${units[product.unit]} disponibles`,
  };
}
export function validQuantity(product: Product, value: number) {
  return (
    Number.isSafeInteger(value) &&
    value > 0 &&
    value <= product.stockUnits &&
    value <= 1_000_000_000_000 &&
    (![0, 4, 5].includes(product.unit) || value % 10000 === 0)
  );
}
export function addQuantity(product: Product, current = 0) {
  const remaining = product.stockUnits - current;
  const increment = [1, 2, 3].includes(product.unit) ? Math.min(10000, remaining) : 10000;
  return increment > 0 && validQuantity(product, current + increment) ? current + increment : null;
}
// Immediate display only. Rust recalculates prices, permissions, tax and stock before checkout.
export function previewAmounts(
  rows: { product: Product; quantityUnits: number; wholesale: boolean }[],
  discount: number,
) {
  const gross = rows.map(
    (r) =>
      (BigInt(r.wholesale ? r.product.wholesaleCents : r.product.priceCents) *
        BigInt(r.quantityUnits) +
        5000n) /
      10000n,
  );
  const sum = gross.reduce((a, b) => a + b, 0n);
  const total = (sum * BigInt(1_000_000 - discount) + 500000n) / 1000000n;
  const shares = gross.map((n) => (sum ? (n * total) / sum : 0n));
  const order = gross
    .map((n, i) => ({ i, remainder: sum ? (n * total) % sum : 0n }))
    .sort((a, b) => (a.remainder === b.remainder ? a.i - b.i : a.remainder > b.remainder ? -1 : 1));
  let left = total - shares.reduce((a, b) => a + b, 0n);
  for (const { i } of order) {
    if (left-- <= 0n) break;
    shares[i] += 1n;
  }
  return {
    totalCents: Number(total),
    lineCents: shares.map(Number),
    discountCents: Number(sum - total),
  };
}

/** Keyboard-wedge readers use fast characters and an Enter/Tab suffix. Never infer a product from a partial name. */
export class ScanBuffer {
  private value = '';
  private last = 0;
  reset() {
    this.value = '';
    this.last = 0;
  }
  key(key: string, time: number, allowed: boolean, modified = false) {
    if (!allowed || modified) {
      this.reset();
      return null;
    }
    if (time - this.last > 70) this.value = '';
    this.last = time;
    if (key === 'Enter' || key === 'Tab') {
      const code = this.value.length >= 4 ? this.value : null;
      this.reset();
      return code;
    }
    if (key.length === 1 && this.value.length < 100) this.value += key;
    else this.reset();
    return null;
  }
}
