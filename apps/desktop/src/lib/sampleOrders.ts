import type { GridColumn, GridRow, PendingChanges } from "@cellar/data-grid";

/**
 * Placeholder data for the `public.orders` tab. The real grid will hydrate
 * itself from a streaming query result over Tauri IPC; until those wires are
 * live, this seed mirrors the design prototype so we can iterate on the grid
 * UX against realistic shape.
 */
export const ORDER_COLUMNS: readonly GridColumn[] = [
  { key: "id", name: "id", type: "uuid", width: 240, pk: true, mono: true },
  { key: "order_number", name: "order_number", type: "text", width: 120, mono: true },
  {
    key: "customer_id",
    name: "customer_id",
    type: "uuid",
    width: 220,
    fk: "customers.id",
    mono: true,
  },
  {
    key: "status",
    name: "status",
    type: "order_status",
    width: 110,
    enum: [
      "pending",
      "paid",
      "fulfilled",
      "shipped",
      "delivered",
      "cancelled",
      "refunded",
    ],
  },
  {
    key: "total_eur",
    name: "total_eur",
    type: "numeric(10,2)",
    width: 110,
    align: "right",
    mono: true,
  },
  { key: "currency", name: "currency", type: "char(3)", width: 70, mono: true },
  { key: "channel", name: "channel", type: "text", width: 90 },
  { key: "country", name: "country", type: "char(2)", width: 70, mono: true },
  { key: "shipping_method", name: "shipping_method", type: "text", width: 130 },
  {
    key: "tax_rate",
    name: "tax_rate",
    type: "numeric(4,3)",
    width: 90,
    align: "right",
    mono: true,
  },
  {
    key: "promo_code",
    name: "promo_code",
    type: "text",
    width: 110,
    mono: true,
    nullable: true,
  },
  { key: "notes", name: "notes", type: "text", width: 200, nullable: true },
  { key: "created_at", name: "created_at", type: "timestamptz", width: 170, mono: true },
  { key: "updated_at", name: "updated_at", type: "timestamptz", width: 170, mono: true },
];

export const ORDERS_TOTAL_ROWS = 1_840_219;

function uuidLike(seed: number, salt: number): string {
  const chars = "0123456789abcdef";
  let out = "";
  let x = (seed * 9301 + salt * 49297 + 1234567) % 0x7fffffff;
  for (let i = 0; i < 32; i++) {
    x = (x * 1103515245 + 12345) & 0x7fffffff;
    out += chars[x & 15];
    if (i === 7 || i === 11 || i === 15 || i === 19) out += "-";
  }
  return out;
}

function fmtTs(d: Date): string {
  const pad = (n: number, w = 2) => String(n).padStart(w, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}+00`;
}

const CHANNELS = ["web", "ios", "android", "pos"] as const;
const COUNTRIES = [
  "DE", "FR", "NL", "IT", "ES", "SE", "PL", "AT", "BE", "DK",
] as const;
const METHODS = ["standard", "express", "pickup", "next-day"] as const;
const STATUSES = [
  "pending",
  "paid",
  "fulfilled",
  "shipped",
  "delivered",
  "cancelled",
  "refunded",
] as const;
const PROMOS: (string | null)[] = [
  null, null, null, null, "WELCOME10", "SUMMER25", "VIP", "FREESHIP",
];

function pick<T>(arr: readonly T[], i: number): T {
  const v = arr[i % arr.length];
  // The runtime length of a non-empty literal ensures this never fails, but
  // noUncheckedIndexedAccess wants the assertion.
  return v as T;
}

/**
 * Deterministic generator — same seed in, same rows out. Keeps screenshots
 * stable and gives reviewers consistent fixtures.
 */
export function makeSampleOrders(count = 60): GridRow[] {
  const out: GridRow[] = [];
  let n = 184_220;
  for (let i = 0; i < count; i++) {
    const status = pick(STATUSES, Math.floor(Math.pow((i * 17) % 97 / 97, 1.8) * STATUSES.length));
    const total = (((i * 73) % 380) + 8 + (i % 7) * 0.13).toFixed(2);
    const tax = (["0.190", "0.200", "0.210", "0.220"] as const)[i % 4];
    const country = pick(COUNTRIES, i);
    const created = new Date(2026, 4, 14 + (i % 12), 8 + (i % 12), (i * 7) % 60, (i * 13) % 60);
    const updated = new Date(created.getTime() + 1000 * 60 * (5 + (i % 700)));
    out.push({
      id: uuidLike(i, 0),
      order_number: `EU-${String(n++).padStart(7, "0")}`,
      customer_id: uuidLike(i, 1),
      status,
      total_eur: total,
      currency: "EUR",
      channel: pick(CHANNELS, i),
      country,
      shipping_method: pick(METHODS, i),
      tax_rate: tax,
      promo_code: pick(PROMOS, i),
      notes:
        i % 9 === 0
          ? "Gift wrap requested"
          : i % 13 === 0
            ? "Customer asked for invoice"
            : null,
      created_at: fmtTs(created),
      updated_at: fmtTs(updated),
    });
  }
  return out;
}

export const SAMPLE_ORDERS: readonly GridRow[] = makeSampleOrders();

/**
 * Seed pending changes so the status bar and pending-bar both have something
 * meaningful to render. Mirrors "4 pending · 1 insert · 2 updates · 1 delete"
 * from the design.
 */
export function makeSamplePendingChanges(): PendingChanges {
  const seed = SAMPLE_ORDERS;
  if (seed.length < 8) return {};
  const r1 = seed[1]!;
  const r2 = seed[4]!;
  const r3 = seed[7]!;
  return {
    [r1.id]: {
      kind: "update",
      edits: {
        status: { from: (r1.status as string) ?? null, to: "shipped" },
      },
    },
    [r2.id]: {
      kind: "update",
      edits: {
        notes: { from: (r2.notes as string | null) ?? null, to: "VIP — overnight" },
        promo_code: { from: (r2.promo_code as string | null) ?? null, to: "VIP" },
      },
    },
    [r3.id]: {
      kind: "delete",
      edits: {},
    },
    "new-row-1": {
      kind: "insert",
      edits: {
        id: { from: null, to: uuidLike(9001, 17) },
        order_number: { from: null, to: "EU-9000001" },
        status: { from: null, to: "pending" },
        total_eur: { from: null, to: "84.20" },
        currency: { from: null, to: "EUR" },
        country: { from: null, to: "DE" },
      },
    },
  };
}
