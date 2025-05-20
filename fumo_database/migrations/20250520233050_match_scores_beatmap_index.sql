-- Add migration script here

create index beatmap_id_index on osu_match_game_scores(beatmap_id);
