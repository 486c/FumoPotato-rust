-- Add migration script here
create table osu_matches (
	id int8 not null,
	name text NOT null,
	start_time timestamp not null,
	end_time timestamp not null,
	constraint osu_match_id primary key (id)
);

create table osu_match_games (
	id int8 not null unique,
	match_id int8 references osu_matches(id),
	beatmap_id int8 not null,
	mods int8 not null,
	mode text not null,
	scoring_kind int2 not null,
	team_kind int2 not null,
	start_time timestamp not null,
	end_time timestamp not null,
	constraint osu_match_game_id primary key (id)
);

create table osu_match_game_scores (
	game_id int8 references osu_match_games(id),
	match_id int8 references osu_matches(id),
	beatmap_id int8 not null,
	user_id int8 not null,
	accuracy float not null,
	mods int8 not null,
	score int8 not null,
	count50 int4 not null,
	count100 int4 not null,
	count300 int4 not null,
	countgeki int4 not null,
	countkatu int4 not null,
	countmiss int4 not null,
	max_combo int4 not null,
	slot int2 not null,
	team text not null,
	pass bool not null,
	pp float
);

create table osu_match_not_found (
	id int8 NOT NULL
);
