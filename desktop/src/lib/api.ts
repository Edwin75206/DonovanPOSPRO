import { invoke } from '@tauri-apps/api/core';
import type { AppearanceState, AppearanceConfig } from './appearance';
export interface Actor {
  id: number;
  username: string;
  name: string;
  role: number;
}
export interface Product {
  id: number;
  sku: string;
  barcode: string | null;
  name: string;
  category: string;
  brand: string;
  size: string;
  aliases: string;
  unit: number;
  costCents: number | null;
  priceCents: number;
  wholesaleCents: number;
  stockUnits: number;
  minimumUnits: number;
  taxBasisPoints: number;
  packName: string;
  packUnits: number;
  active: boolean;
}
export interface Preferences {
  tutorialStatus: 'pending' | 'skipped' | 'in_progress' | 'completed';
  tutorialStep: number;
  theme: 'system' | 'light' | 'dark';
  reducedMotion: boolean;
}
export interface Settings {
  id: number;
  storeName: string;
  logoData: string | null;
  address: string;
  phone: string;
  email: string;
  taxId: string;
  ticketFooter: string;
  folioPrefix: string;
  defaultTaxBasisPoints: number;
  ticketWidthMm: number;
  printerName: string;
  backupDirectory: string;
  automaticBackups: boolean;
  backupIntervalHours: number;
  darkMode: boolean;
  reducedMotion: boolean;
}
export interface Bootstrap {
  needsSetup: boolean;
  actor: Actor | null;
  settings: Settings | null;
  preferences: Preferences | null;
  appearance: AppearanceState | null;
  profile: { storeName: string; accent: string; highlight: string };
  version: string;
  demo: boolean;
  locked: boolean;
  storageError: UiFailure | null;
  databaseId?: string;
  sessionExpiresAt?: number | null;
}
export interface DashboardData {
  from: string;
  to: string;
  dayCents: number;
  weekCents: number;
  monthCents: number;
  grossCents: number;
  returnsCents: number;
  totalCents: number;
  previousCents: number;
  profitCents: number | null;
  operations: number;
  dayOperations: number;
  receivableCents: number;
  creditGrantedCents: number;
  cashCents: number;
  cashSession: CashSession | null;
  lastCashClose: CashSession | null;
  trend: { date: string; totalCents: number }[];
  methods: { method: number; totalCents: number }[];
  topProducts: {
    productId: number;
    name: string;
    unit: number;
    quantityUnits: number;
    totalCents: number;
  }[];
  categories: { name: string; totalCents: number }[];
  lowStock: Pick<Product, 'id' | 'sku' | 'name' | 'unit' | 'stockUnits' | 'minimumUnits'>[];
  stockAlerts: { emptyCount: number; lowCount: number };
  recentSales: Sale[];
  recentPurchases: {
    id: number;
    folio: string;
    supplierName: string;
    totalCents: number;
    createdAt: string;
  }[];
  activity: {
    id: number;
    createdAt: string;
    action: string;
    reference: string;
    userName: string | null;
  }[];
  stockMovements: {
    id: number;
    productName: string;
    kind: string;
    deltaUnits: number;
    unit: number;
    createdAt: string;
  }[];
  users: (Actor & { active: boolean; lastAccess: number | null })[];
  lastBackup: { createdAt: string; fileBytes: number; kind: string } | null;
  lastIntegrity: string | null;
  databaseBytes: number;
  schemaVersion: number;
}
export interface Item {
  productId: number;
  quantityUnits: number;
  wholesale: boolean;
}
export interface ChargeLine extends Omit<Item, 'wholesale'> {
  sku: string;
  name: string;
  category: string;
  unit: number;
  priceCents: number;
  costCents: number | null;
  taxBasisPoints: number;
  totalCents: number;
  taxCents: number;
}
export interface Charge {
  lines: ChargeLine[];
  totalCents: number;
  taxCents: number;
  discountCents: number;
}
export interface Sale {
  id: number;
  folio: string;
  requestId: string;
  createdAt: string;
  userId: number;
  cashier: string;
  customerId: number | null;
  customerName: string;
  totalCents: number;
  taxCents: number;
  discountCents: number;
  changeCents: number;
  tenderedCents: number;
  cancelled: boolean;
}
export interface SaleDetail {
  sale: Sale;
  lines: (ChargeLine & { id: number; returnedUnits: number; refundedCents: number })[];
  payments: Payment[];
  returns: Record<string, unknown>[];
}
export interface Payment {
  method: number;
  amountCents: number;
  reference: string;
}
export interface Checkout {
  requestId: string;
  items: Item[];
  payments: Payment[];
  tenderedCents: number;
  customerId: number | null;
  dueDate: string | null;
  discountParts: number;
  quoteId: number | null;
  heldSaleId: number | null;
}
export interface ProductPage {
  items: Product[];
  total: number;
  page: number;
  pageSize: number;
  categories: { name: string; count: number }[];
}
export interface StockMovement {
  id: number;
  createdAt: string;
  kind: string;
  reference: string;
  reason: string;
  deltaUnits: number;
  balanceUnits: number;
}
export interface CashSession {
  id: number;
  folio: string;
  openedAt: string;
  closedAt: string | null;
  openingCents: number;
  expectedCents: number | null;
  countedCents: number | null;
  notes: string;
}
export interface CashMovement {
  id: number;
  createdAt: string;
  amountCents: number;
  kind: string;
  reference: string;
  reason: string;
}
export interface CashOverview {
  open: CashSession | null;
  expectedCents: number;
  movements: CashMovement[];
  history: CashSession[];
}
export interface Setup {
  storeName: string;
  adminUsername: string;
  adminName: string;
  adminPassword: string;
  sellerUsername: string;
  sellerName: string;
  sellerPassword: string;
  address: string;
  phone: string;
  taxId: string;
  ticketWidthMm: number;
}
export interface UiFailure {
  code: string;
  message: string;
  incidentId: string;
  retryable: boolean;
}
export interface BackupRecord {
  id: number;
  createdAt: string;
  filePath: string;
  kind: string;
  successful: boolean;
  fileBytes: number;
  sha256: string;
  result: string;
}
export interface DataOverview {
  path: string;
  root: string;
  bytes: number;
  freeBytes: number;
  schemaVersion: number;
  lastIntegrity: string | null;
  settings: { backupDirectory: string; automaticBackups: boolean; backupIntervalHours: number };
  migrations: { version: number; description: string; appliedAt: string }[];
  history: BackupRecord[];
  counts: { products: number; sales: number; customers: number; users: number };
}
export interface RecoveryPreview {
  planId: string;
  fileName: string;
  storeName: string;
  schemaVersion: number;
  targetVersion: number;
  legacy: boolean;
  createdAt: string | null;
  bytes: number;
  hasLogo: boolean;
  authorization: 'current-admin' | 'backup-admin';
  adminAccounts: { username: string; name: string }[];
  counts: { products: number; sales: number; customers: number; users: number; quotes: number };
}
export interface Diagnostic {
  time: string;
  code: string;
  module: string;
  operation: string;
  incidentId: string;
  message: string | null;
}
export interface ImportBook {
  fileName: string;
  sheets: string[];
  sheet: string;
  headers: string[];
  rows: number;
  sample: { row: number; values: string[] }[];
  mapping: Record<string, number>;
  fields: { key: string; label: string }[];
}
export interface ImportOptions {
  mapping: Record<string, number>;
  mode: 'add' | 'update' | 'upsert';
  prices: 'keep' | 'replace';
  stock: 'keep' | 'replace' | 'add';
}
export interface ImportPreview {
  planId: string;
  valid: boolean;
  total: number;
  added: number;
  updated: number;
  priceChanges: number;
  stockChanges: number;
  errorCount: number;
  errors: { id: number; row: number; message: string }[];
  rows: {
    id: number;
    row: number;
    sku: string;
    name: string;
    action: string;
    oldPriceCents: number | null;
    priceCents: number;
    oldStockUnits: number | null;
    stockUnits: number;
    unit: number;
  }[];
}
export class PosError extends Error {
  constructor(public failure: UiFailure) {
    super(failure.message);
    this.name = 'PosError';
  }
}
export const normalizeError = (error: unknown): UiFailure => {
  if (error instanceof PosError) return error.failure;
  if (error && typeof error === 'object' && 'code' in error && 'message' in error)
    return error as UiFailure;
  return {
    code: 'UNEXPECTED',
    message: 'Ocurrió un problema y no pudimos completar esta operación. Intenta nuevamente.',
    incidentId: crypto.randomUUID().slice(0, 8),
    retryable: true,
  };
};
// This map is the frontend contract. Only named Rust services can access business data.
export interface Calls {
  chooseImportFile: [undefined, ImportBook | null];
  importSheet: [{ sheet: string }, ImportBook];
  validateImport: [ImportOptions, ImportPreview];
  commitImport: [
    { planId: string },
    { added: number; updated: number; priceChanges: number; stockChanges: number },
  ];
  exportProducts: [{ template: boolean }, { path: string; bytes: number } | null];
  dataOverview: [undefined, DataOverview];
  verifyDatabase: [undefined, { valid: boolean; checkedAt: string; schemaVersion: number }];
  createBackup: [
    { folder: string },
    { path: string; bytes: number; sha256: string; createdAt: string },
  ];
  chooseBackupFolder: [undefined, string | null];
  chooseRecoverySource: [undefined, RecoveryPreview | null];
  confirmRecovery: [{ planId: string; username: string; password: string }, Bootstrap];
  cancelRecovery: [undefined, null];
  newDatabase: [{ password: string }, Bootstrap];
  saveBackupSettings: [
    { folder: string; enabled: boolean; hours: number; password: string },
    DataOverview,
  ];
  diagnostics: [undefined, Diagnostic[]];
  dashboard: [{ from: string; to: string }, DashboardData];
  saveAppearance: [
    {
      config: AppearanceConfig;
      backgroundAction: 'keep' | 'replace' | 'remove';
      backgroundData: string | null;
    },
    AppearanceState,
  ];
  chooseAppearanceImage: [undefined, { data: string } | null];
  bootstrap: [undefined, Bootstrap];
  retryStorage: [undefined, Bootstrap];
  setup: [Setup, Bootstrap];
  login: [{ username: string; password: string }, Bootstrap];
  logout: [undefined, Bootstrap];
  lock: [undefined, Bootstrap];
  unlock: [{ password: string }, Bootstrap];
  demoLogin: [{ role: number }, Bootstrap];
  products: [{ search: string; category: string; status: string; page: number }, ProductPage];
  product: [{ id: number }, Product];
  inventory: [
    {
      search: string;
      category: string;
      status: string;
      page: number;
      sort: string;
      descending: boolean;
    },
    ProductPage,
  ];
  saveProduct: [Product, Product];
  adjustStock: [{ id: number; delta: number; reason: string }, Product];
  stockHistory: [{ id: number }, StockMovement[]];
  price: [{ items: Item[]; discountParts: number }, Charge];
  checkout: [Checkout, SaleDetail];
  sale: [{ id: number }, SaleDetail];
  sales: [{ from: string; to: string; page: number }, Sale[]];
  cashOverview: [undefined, CashOverview];
  cashOpen: [{ openingCents: number }, CashOverview];
  cashMovement: [{ amountCents: number; reason: string }, CashOverview];
  cashClose: [{ countedCents: number; notes: string }, CashOverview];
  users: [undefined, (Actor & { active: boolean })[]];
  saveUser: [
    {
      id: number;
      username: string;
      name: string;
      role: number;
      active: boolean;
      password: string;
      adminPassword: string;
    },
    { saved: boolean; reauthenticate: boolean },
  ];
  preferences: [
    { status: string; step: number; theme: string; reducedMotion: boolean },
    Preferences,
  ];
}
export async function api<K extends keyof Calls>(
  type: K,
  ...args: Calls[K][0] extends undefined ? [payload?: undefined] : [payload: Calls[K][0]]
): Promise<Calls[K][1]> {
  try {
    return await invoke('execute', {
      request: args[0] === undefined ? { type } : { type, payload: args[0] },
    });
  } catch (error) {
    const failure = normalizeError(error);
    if (failure.code === 'SESSION_EXPIRED' || failure.code === 'SESSION_LOCKED')
      window.dispatchEvent(new CustomEvent('session-ended', { detail: failure }));
    throw new PosError(failure);
  }
}
export const can = (actor: Actor, permission: string) =>
  actor.role === 0 ||
  ['sell', 'quote', 'cash'].includes(permission) ||
  (actor.role === 2 && ['catalog', 'stock', 'purchase', 'credit', 'discount'].includes(permission));
