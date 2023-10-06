CREATE TABLE [IF NOT EXISTS] products (
    id serial primary key,
    name varchar(100) not null
);

CREATE TABLE [IF NOT EXISTS] prices (
    product_id int not null,
    receipt_id int not null,
    count int not null,
    unit_price float not null,
    foreign key (product_id) references products (id),
    foreign key (receipt_id) references receipts (id),
    primary key (product_id, receipt_id)
);

CREATE TABLE [IF NOT EXISTS] receipts (
    id serial primary key,
    merchant_name varchar(100) not null,
    paid_at timestamp not null
);