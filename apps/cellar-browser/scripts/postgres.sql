CREATE TABLE customers (id integer PRIMARY KEY, name text NOT NULL, email text, region text);
CREATE TABLE orders (id integer PRIMARY KEY, customer_id integer REFERENCES customers(id), status text, total numeric(12,2), created_at date, metadata jsonb);
INSERT INTO customers SELECT n, 'Demo customer ' || lpad(n::text,3,'0'), 'customer' || n || '@example.test', (ARRAY['NZ','AU','UK'])[n % 3 + 1] FROM generate_series(1,60) n;
INSERT INTO orders SELECT n, n % 60 + 1, (ARRAY['paid','pending','refunded'])[n % 3 + 1], n * 12.75, DATE '2026-09-01' + (n % 28), jsonb_build_object('source','synthetic','itemCount',n % 5 + 1) FROM generate_series(1,350) n;
CREATE VIEW paid_orders AS SELECT * FROM orders WHERE status='paid';
CREATE ROLE cellar_reader LOGIN;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO cellar_reader;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO cellar_reader;
