-- Add up migration script here
BEGIN;

CREATE TEMPORARY TABLE tmp AS 
SELECT receipts.id, receipts.merchant_name, receipts.paid_at FROM (
    SELECT MIN(id) AS id, paid_at FROM receipts 
    GROUP BY paid_at
) lft
INNER JOIN receipts ON receipts.id = lft.id;

DELETE FROM receipts;

ALTER TABLE receipts 
ADD CONSTRAINT unique_paid_at UNIQUE (paid_at);

INSERT INTO receipts (id, merchant_name, paid_at) SELECT tmp.id, tmp.merchant_name, tmp.paid_at FROM tmp;

DROP TABLE tmp;

COMMIT;