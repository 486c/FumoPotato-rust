create table twitch_streamers(
	name varchar not null,
	id bigint not null,
	online boolean not null default false,
	primary key(id),
);

create table twitch_tracking(
	channel_id bigint not null,
	id bigint not null,
	constraint fk_id
		foreign key(id)
			references twitch_streamers(id)
);
