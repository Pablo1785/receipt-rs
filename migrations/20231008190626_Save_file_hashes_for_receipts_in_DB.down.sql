-- Add down migration script here
ALTER TABLE receipts 
DROP COLUMN file_sha256;
