-- Add up migration script here
CREATE TABLE users (
    id serial not null primary key,
    email text unique not null,
    google_drive_access_token text,
    google_drive_access_token_created_at timestamptz,
    google_drive_refresh_token text,
    google_drive_refresh_token_created_at timestamptz
);