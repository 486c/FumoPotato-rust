-- Add migration script here
CREATE TABLE IF NOT EXISTS osu_users(
	osu_id bigint not null,
	discord_id bigint not null,
	primary key(discord_id)
)
