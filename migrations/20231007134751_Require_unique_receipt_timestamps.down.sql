-- Add down migration script here
ALTER TABLE receipts 
DROP CONSTRAINT unique_paid_at;