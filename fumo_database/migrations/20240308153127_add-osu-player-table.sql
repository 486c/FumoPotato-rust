-- Add migration script here
create table osu_players(
	osu_id int8 primary key not null,
	osu_username varchar not null
);

alter table osu_tracking drop constraint fk_osu_id;

ALTER TABLE osu_tracking add constraint fk_osu_id foreign key (osu_id) references osu_players(osu_id);
ALTER TABLE osu_tracked_users add constraint fk_osu_id foreign key (osu_id) references osu_players(osu_id);
