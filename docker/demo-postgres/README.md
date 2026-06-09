# Demo Postgres

A throwaway Postgres instance with a small, realistic **e-commerce** dataset for
taking Cellar screenshots. All data is procedurally generated — no real PII.

## Start

```bash
cd docker/demo-postgres
docker compose up -d
```

First boot runs `initdb/01-schema.sql` then `initdb/02-seed.sql` automatically.

## Connect

Add a connection in Cellar (or use `psql`) with:

| field    | value         |
| -------- | ------------- |
| Host     | `localhost`   |
| Port     | `5432`        |
| Database | `cellar_demo` |
| User     | `cellar`      |
| Password | `cellar`      |
| SSL mode | `disable`     |

```bash
psql "postgresql://cellar:cellar@localhost:5432/cellar_demo"
```

## What's inside

Schema `public`:

| object                  | kind  | rows  | notes                                   |
| ----------------------- | ----- | ----- | --------------------------------------- |
| `categories`            | table | 8     |                                         |
| `products`              | table | 120   | `numeric` price, `jsonb` attributes     |
| `customers`             | table | 200   | `jsonb` metadata, `is_vip` flag         |
| `orders`                | table | 600   | `order_status` enum, FK → customers     |
| `order_items`           | table | ~1450 | 1–4 line items per order                |
| `payments`              | table | ~520  | `payment_method` / `payment_status` enums |
| `customer_order_summary`| view  | —     | per-customer order count + spend        |
| `product_sales`         | view  | —     | per-product units sold + revenue        |

Enum types: `order_status`, `payment_method`, `payment_status`. Foreign keys,
indexes, and column comments are all set so the schema browser has plenty to show.

## Re-seed (fresh random data)

The init scripts only run when the data volume is empty, so wiping the volume
gives a fresh dataset:

```bash
docker compose down -v && docker compose up -d
```

The seed uses a fixed `setseed(0.42)`, so a re-seed reproduces the same data.
Change that value in `initdb/02-seed.sql` for a different (still deterministic) set.

## Stop / remove

```bash
docker compose stop        # pause, keep data
docker compose down        # remove container, keep data volume
docker compose down -v     # remove container AND data
```
