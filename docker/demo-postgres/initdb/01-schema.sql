-- Cellar demo schema: a small, recognizable e-commerce model.
-- Rich on purpose — enums, jsonb, numeric, timestamps, FKs, indexes, comments,
-- and views — so the schema browser and grid have interesting things to show.

SET client_min_messages = warning;

-- ---------------------------------------------------------------------------
-- Enumerated types (render as type badges in the grid)
-- ---------------------------------------------------------------------------
CREATE TYPE order_status AS ENUM (
  'pending', 'paid', 'shipped', 'delivered', 'cancelled', 'refunded'
);

CREATE TYPE payment_method AS ENUM (
  'card', 'paypal', 'bank_transfer', 'apple_pay', 'google_pay'
);

CREATE TYPE payment_status AS ENUM (
  'pending', 'completed', 'failed', 'refunded'
);

-- ---------------------------------------------------------------------------
-- Tables
-- ---------------------------------------------------------------------------
CREATE TABLE categories (
  id          serial PRIMARY KEY,
  name        text        NOT NULL,
  slug        text        NOT NULL UNIQUE,
  created_at  timestamptz NOT NULL DEFAULT now()
);
COMMENT ON TABLE  categories      IS 'Product categories shown in the storefront nav.';
COMMENT ON COLUMN categories.slug IS 'URL-safe identifier, unique per category.';

CREATE TABLE products (
  id           serial PRIMARY KEY,
  category_id  integer     NOT NULL REFERENCES categories(id),
  sku          text        NOT NULL UNIQUE,
  name         text        NOT NULL,
  description  text,
  price        numeric(10,2) NOT NULL CHECK (price >= 0),
  in_stock     integer     NOT NULL DEFAULT 0,
  is_active    boolean     NOT NULL DEFAULT true,
  attributes   jsonb       NOT NULL DEFAULT '{}'::jsonb,
  created_at   timestamptz NOT NULL DEFAULT now()
);
COMMENT ON TABLE  products            IS 'Sellable catalog items.';
COMMENT ON COLUMN products.attributes IS 'Free-form product spec (color, size, weight, ...).';

CREATE TABLE customers (
  id              serial PRIMARY KEY,
  first_name      text        NOT NULL,
  last_name       text        NOT NULL,
  email           text        NOT NULL UNIQUE,
  country         text        NOT NULL,
  city            text        NOT NULL,
  signup_date     date        NOT NULL,
  lifetime_value  numeric(12,2) NOT NULL DEFAULT 0,
  is_vip          boolean     NOT NULL DEFAULT false,
  metadata        jsonb       NOT NULL DEFAULT '{}'::jsonb
);
COMMENT ON TABLE customers IS 'People who have signed up — all data here is synthetic.';

CREATE TABLE orders (
  id           serial PRIMARY KEY,
  customer_id  integer       NOT NULL REFERENCES customers(id),
  status       order_status  NOT NULL DEFAULT 'pending',
  currency     char(3)       NOT NULL DEFAULT 'USD',
  total        numeric(12,2) NOT NULL DEFAULT 0,
  placed_at    timestamptz   NOT NULL DEFAULT now(),
  shipped_at   timestamptz
);
COMMENT ON TABLE orders IS 'Customer orders; total is rolled up from order_items.';

CREATE TABLE order_items (
  id          serial PRIMARY KEY,
  order_id    integer       NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
  product_id  integer       NOT NULL REFERENCES products(id),
  quantity    integer       NOT NULL CHECK (quantity > 0),
  unit_price  numeric(10,2) NOT NULL
);
COMMENT ON TABLE order_items IS 'Line items belonging to an order.';

CREATE TABLE payments (
  id        serial PRIMARY KEY,
  order_id  integer        NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
  method    payment_method NOT NULL,
  amount    numeric(12,2)  NOT NULL,
  status    payment_status NOT NULL DEFAULT 'pending',
  paid_at   timestamptz
);
COMMENT ON TABLE payments IS 'Payment attempts against an order.';

-- ---------------------------------------------------------------------------
-- Indexes
-- ---------------------------------------------------------------------------
CREATE INDEX idx_products_category   ON products(category_id);
CREATE INDEX idx_customers_country   ON customers(country);
CREATE INDEX idx_orders_customer     ON orders(customer_id);
CREATE INDEX idx_orders_status       ON orders(status);
CREATE INDEX idx_orders_placed_at    ON orders(placed_at);
CREATE INDEX idx_order_items_order   ON order_items(order_id);
CREATE INDEX idx_order_items_product ON order_items(product_id);
CREATE INDEX idx_payments_order      ON payments(order_id);

-- ---------------------------------------------------------------------------
-- Views
-- ---------------------------------------------------------------------------
CREATE VIEW customer_order_summary AS
SELECT c.id,
       c.first_name,
       c.last_name,
       c.email,
       c.country,
       count(o.id)                  AS order_count,
       coalesce(sum(o.total), 0)    AS total_spent,
       max(o.placed_at)             AS last_order_at
FROM customers c
LEFT JOIN orders o ON o.customer_id = c.id
GROUP BY c.id;

CREATE VIEW product_sales AS
SELECT p.id,
       p.sku,
       p.name,
       cat.name                                AS category,
       count(oi.id)                            AS times_ordered,
       coalesce(sum(oi.quantity), 0)           AS units_sold,
       coalesce(sum(oi.quantity * oi.unit_price), 0) AS revenue
FROM products p
JOIN categories cat ON cat.id = p.category_id
LEFT JOIN order_items oi ON oi.product_id = p.id
GROUP BY p.id, cat.name;
