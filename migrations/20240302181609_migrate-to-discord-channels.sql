-- Add migration script here
insert into discord_channels select distinct channel_id from twitch_tracking;

create table twitch_streamers_new (
	twitch_id int8 primary key not NULL,
	online bool default false not null
	--CONSTRAINT twitch_streamers_pkey PRIMARY KEY (twitch_id)
);

create table twitch_tracking_new (
	channel_id bigint,
	twitch_id bigint,
	constraint fk_channel_id foreign KEY("channel_id") REFERENCES discord_channels(channel_id),
	constraint fk_twitch_id foreign KEY("twitch_id") REFERENCES twitch_streamers_new(twitch_id)
	
);

insert into twitch_streamers_new select * from twitch_streamers;
insert into twitch_tracking_new select * from twitch_tracking;

drop table twitch_tracking;
drop table twitch_streamers;

alter table twitch_tracking_new rename to twitch_tracking;
alter table twitch_streamers_new rename to twitch_streamers;
