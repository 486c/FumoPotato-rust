-- Add migration script here
CREATE TABLE IF NOT EXISTS twitch_streamers(
	name varchar not null,
	id bigint not null,
	online boolean not null default false,
	primary key(id)
);

CREATE TABLE IF NOT EXISTS twitch_tracking(
	channel_id bigint not null,
	id bigint not null,
	constraint fk_id
		foreign key(id)
			references twitch_streamers(id)
)
