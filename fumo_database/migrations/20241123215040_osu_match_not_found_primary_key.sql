-- Add migration script here

ALTER TABLE osu_match_not_found RENAME TO osu_match_not_found_temp;

create table osu_match_not_found (
	id int8 NOT NULL UNIQUE,
	constraint osu_match_not_found_id primary key (id)
);

INSERT INTO osu_match_not_found (id)
SELECT id
FROM osu_match_not_found_temp 
ON CONFLICT (id) DO NOTHING;

DROP TABLE osu_match_not_found_temp;
