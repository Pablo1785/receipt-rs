-- Add up migration script here
CREATE TABLE raw_results (
    id serial not null primary key,
    result_json text not null,
    sha256_digest text not null
);