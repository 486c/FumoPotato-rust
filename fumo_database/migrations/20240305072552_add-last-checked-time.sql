-- Add migration script here

create table osu_tracked_users(
	osu_id int8 primary key not NULL,
	last_checked timestamp not null
);

ALTER TABLE osu_tracking add constraint fk_osu_id foreign key (osu_id) references osu_tracked_users(osu_id);
