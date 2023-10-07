-- Add up migration script here
BEGIN;

CREATE TEMPORARY TABLE tmp_receipts AS 
SELECT receipts.id, receipts.merchant_name, receipts.paid_at FROM (
    SELECT MIN(id) AS id, paid_at FROM receipts 
    GROUP BY paid_at
) lft
INNER JOIN receipts ON receipts.id = lft.id;


CREATE TEMPORARY TABLE tmp_prices AS 
SELECT prices.* FROM prices
INNER JOIN tmp_receipts ON prices.receipt_id = tmp_receipts.id;

DELETE FROM prices;

DELETE FROM receipts;

ALTER TABLE receipts 
ADD CONSTRAINT unique_paid_at UNIQUE (paid_at);

INSERT INTO receipts (id, merchant_name, paid_at) 
SELECT tmp_receipts.id, tmp_receipts.merchant_name, tmp_receipts.paid_at FROM tmp_receipts;

INSERT INTO prices (receipt_id, product_id, count, unit_price) 
SELECT tmp_prices.receipt_id, tmp_prices.product_id, tmp_prices.count, tmp_prices.unit_price FROM tmp_prices;

DROP TABLE tmp_receipts;

COMMIT;