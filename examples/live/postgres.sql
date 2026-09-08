CREATE TABLE connector_fixture (
    id integer PRIMARY KEY,
    label text NOT NULL,
    exact_value numeric NOT NULL,
    nullable_value text
);
INSERT INTO connector_fixture VALUES
    (1, 'first', 12345678901234567890.123456789, NULL),
    (2, 'second', 2.5, 'present'),
    (3, 'third', 3, NULL);
CREATE ROLE connector_reader LOGIN;
GRANT CONNECT ON DATABASE engineering TO connector_reader;
GRANT USAGE ON SCHEMA public TO connector_reader;
GRANT SELECT ON connector_fixture TO connector_reader;
