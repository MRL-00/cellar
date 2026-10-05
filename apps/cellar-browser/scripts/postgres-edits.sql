CREATE ROLE cellar_editor LOGIN;
CREATE DATABASE cellar_edit_fixture OWNER cellar_editor;
\connect cellar_edit_fixture
SET ROLE cellar_editor;
CREATE TABLE sample(id integer PRIMARY KEY, name text NOT NULL, payload jsonb, amount numeric(12,2));
INSERT INTO sample VALUES(1,'Original','{"safe":true}',12.75),(2,'Delete me',NULL,20.00);
