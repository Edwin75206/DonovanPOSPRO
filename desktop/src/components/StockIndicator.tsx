import { AlertTriangle, PackageCheck, PackageX } from 'lucide-react';
import type { Product } from '@/lib/api';
import { stockState } from '@/lib/register';

export function StockIndicator({ product, reserved = 0 }: { product: Product; reserved?: number }) {
  const stock = stockState(product, reserved);
  const Icon =
    stock.state === 'empty' ? PackageX : stock.state === 'low' ? AlertTriangle : PackageCheck;
  return (
    <span className={`stock-indicator stock-${stock.state}`}>
      <Icon size={14} aria-hidden="true" />
      {stock.label}
    </span>
  );
}
