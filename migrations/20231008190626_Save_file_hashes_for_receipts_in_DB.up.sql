-- Add up migration script here
ALTER TABLE receipts 
ADD COLUMN file_sha256 char(64) unique;