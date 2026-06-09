-- Synthetic seed data. Everything below is procedurally generated with random()
-- so there is zero real-world PII. Volumes are tuned to look real but load fast.
--
-- NOTE: random() picks are computed in a subquery SELECT list *over*
-- generate_series (so they evaluate once per row), and the per-order line-item
-- pick uses a correlated LATERAL (references o.id). An *uncorrelated* LATERAL
-- would have its random() folded to a single value and every row would match.

SET client_min_messages = warning;

-- Fixed seed -> the same demo data every time you re-seed (down -v && up -d).
SELECT setseed(0.42);

-- ---------------------------------------------------------------------------
-- Categories (fixed set -> ids 1..8)
-- ---------------------------------------------------------------------------
INSERT INTO categories (name, slug) VALUES
  ('Audio',       'audio'),
  ('Wearables',   'wearables'),
  ('Home Office', 'home-office'),
  ('Cameras',     'cameras'),
  ('Gaming',      'gaming'),
  ('Accessories', 'accessories'),
  ('Networking',  'networking'),
  ('Storage',     'storage');

-- ---------------------------------------------------------------------------
-- Products (~120)
-- ---------------------------------------------------------------------------
INSERT INTO products (category_id, sku, name, description, price, in_stock, is_active, attributes, created_at)
SELECT
  category_id,
  'SKU-' || lpad(g::text, 5, '0'),
  adj || ' ' || noun,
  'A ' || lower(adj) || ' ' || lower(noun) || ' for everyday use.',
  price,
  in_stock,
  is_active,
  jsonb_build_object('color', color, 'weight_g', weight_g, 'warranty_months', warranty),
  created_at
FROM (
  SELECT
    g,
    1 + floor(random() * 8)::int AS category_id,
    (ARRAY['Aurora','Nimbus','Vertex','Quartz','Pulse','Atlas','Lumen','Echo',
           'Pixel','Nova','Forge','Drift','Orbit','Halo','Cobalt','Zenith'])[1 + floor(random() * 16)::int] AS adj,
    (ARRAY['Headphones','Watch','Desk Lamp','Camera','Controller','Keyboard',
           'Mouse','Router','SSD','Earbuds','Monitor','Webcam','Speaker','Hub',
           'Microphone','Tripod'])[1 + floor(random() * 16)::int] AS noun,
    round((5 + random() * 995)::numeric, 2) AS price,
    floor(random() * 500)::int AS in_stock,
    random() < 0.92 AS is_active,
    (ARRAY['black','white','silver','blue','red','graphite'])[1 + floor(random() * 6)::int] AS color,
    floor(50 + random() * 1950)::int AS weight_g,
    (ARRAY[12, 24, 36])[1 + floor(random() * 3)::int] AS warranty,
    now() - (random() * 720 || ' days')::interval AS created_at
  FROM generate_series(1, 120) g
) r;

-- ---------------------------------------------------------------------------
-- Customers (~200 -> ids 1..200)
-- ---------------------------------------------------------------------------
INSERT INTO customers (first_name, last_name, email, country, city, signup_date, lifetime_value, is_vip, metadata)
SELECT
  fn,
  ln,
  lower(fn || '.' || ln || g || '@example.com'),
  (ARRAY['United States','United Kingdom','Germany','France','Australia',
         'Canada','Japan','Netherlands','Spain','Sweden'])[loc],
  (ARRAY['New York','London','Berlin','Paris','Sydney',
         'Toronto','Tokyo','Amsterdam','Madrid','Stockholm'])[loc],
  signup_date,
  lifetime_value,
  is_vip,
  jsonb_build_object('segment', segment, 'newsletter', newsletter, 'referral', referral)
FROM (
  SELECT
    g,
    (ARRAY['Ava','Liam','Mia','Noah','Emma','Ethan','Olivia','Lucas','Sofia',
           'Mason','Isla','Leo','Aria','Finn','Maya','Hugo','Nora','Felix',
           'Ruby','Theo'])[1 + floor(random() * 20)::int] AS fn,
    (ARRAY['Smith','Johnson','Williams','Brown','Jones','Garcia','Miller',
           'Davis','Rodriguez','Martinez','Hernandez','Lopez','Wilson','Anderson',
           'Thomas','Taylor','Moore','Jackson','Martin','Lee'])[1 + floor(random() * 20)::int] AS ln,
    1 + floor(random() * 10)::int AS loc,
    (DATE '2022-01-01' + (random() * 1250)::int) AS signup_date,
    round((random() * 8000)::numeric, 2) AS lifetime_value,
    random() < 0.15 AS is_vip,
    (ARRAY['consumer','smb','enterprise'])[1 + floor(random() * 3)::int] AS segment,
    random() < 0.6 AS newsletter,
    (ARRAY['organic','ads','partner','referral'])[1 + floor(random() * 4)::int] AS referral
  FROM generate_series(1, 200) g
) r;

-- ---------------------------------------------------------------------------
-- Orders (~600 -> ids 1..600). total filled in after line items exist.
-- ---------------------------------------------------------------------------
INSERT INTO orders (customer_id, status, currency, total, placed_at, shipped_at)
SELECT
  customer_id,
  status,
  currency,
  0,
  placed_at,
  CASE WHEN status IN ('shipped','delivered')
       THEN placed_at + (random() * 9 || ' days')::interval
       ELSE NULL END
FROM (
  SELECT
    g,
    1 + floor(random() * 200)::int AS customer_id,
    -- weighted toward delivered/paid so the shop looks healthy
    (ARRAY['pending','paid','paid','shipped','delivered','delivered','delivered',
           'cancelled','refunded'])[1 + floor(random() * 9)::int]::order_status AS status,
    (ARRAY['USD','EUR','GBP','AUD'])[1 + floor(random() * 4)::int]::char(3) AS currency,
    now() - (random() * 600 || ' days')::interval AS placed_at
  FROM generate_series(1, 600) g
) r;

-- ---------------------------------------------------------------------------
-- Order items (1..4 distinct products per order; correlated LATERAL)
-- ---------------------------------------------------------------------------
INSERT INTO order_items (order_id, product_id, quantity, unit_price)
SELECT o.id, p.id, 1 + floor(random() * 4)::int, p.price
FROM orders o
CROSS JOIN LATERAL (
  SELECT id, price
  FROM products
  WHERE o.id IS NOT NULL          -- correlation hook: forces per-order evaluation
  ORDER BY random()
  LIMIT 1 + floor(random() * 4)::int
) p;

-- Roll the line-item totals back up onto the order.
UPDATE orders o
SET total = s.total
FROM (
  SELECT order_id, round(sum(quantity * unit_price), 2) AS total
  FROM order_items
  GROUP BY order_id
) s
WHERE o.id = s.order_id;

-- ---------------------------------------------------------------------------
-- Payments (one per non-pending order; status tracks the order)
-- ---------------------------------------------------------------------------
INSERT INTO payments (order_id, method, amount, status, paid_at)
SELECT
  o.id,
  method,
  o.total,
  CASE
    WHEN o.status = 'refunded'  THEN 'refunded'
    WHEN o.status = 'cancelled' THEN 'failed'
    ELSE 'completed'
  END::payment_status,
  CASE WHEN o.status = 'cancelled' THEN NULL
       ELSE o.placed_at + (random() * 2 || ' days')::interval END
FROM orders o
CROSS JOIN LATERAL (
  SELECT (ARRAY['card','card','paypal','bank_transfer','apple_pay','google_pay'])[1 + floor(random() * 6)::int]::payment_method AS method
  WHERE o.id IS NOT NULL
) m
WHERE o.status <> 'pending';

-- Refresh lifetime_value from real order history for a believable column.
UPDATE customers c
SET lifetime_value = s.spent
FROM (
  SELECT customer_id, round(sum(total), 2) AS spent
  FROM orders
  WHERE status IN ('paid','shipped','delivered')
  GROUP BY customer_id
) s
WHERE c.id = s.customer_id;
