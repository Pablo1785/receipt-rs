-- Add migration script here
SET timezone = 'Europe/Copenhagen';

CREATE TABLE products (
    id serial primary key,
    name text unique not null
);

CREATE TABLE receipts (
    id serial not null primary key,
    merchant_name text not null,
    paid_at timestamptz not null
);

CREATE TABLE prices (
    product_id int not null,
    receipt_id int not null,
    count float not null,
    unit_price float not null,
    foreign key (product_id) references products (id),
    foreign key (receipt_id) references receipts (id),
    primary key (product_id, receipt_id)
);