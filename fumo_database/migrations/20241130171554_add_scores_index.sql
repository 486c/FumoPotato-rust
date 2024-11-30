-- Add migration script here

create index user_id_index on osu_match_game_scores(user_id)
