-- Add migration script here

CREATE TABLE osu_username_kv (
	osu_id bigint not null,
	osu_username text not null,
	primary key(osu_id)
)
