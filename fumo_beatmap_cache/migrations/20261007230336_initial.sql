-- Add migration script here

CREATE TABLE IF NOT EXISTS beatmaps(
	id bigint primary key,
	data BYTEA not null

);
