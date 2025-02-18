-- Add migration script here

ALTER TABLE osu_match_game_scores ALTER COLUMN slot TYPE int2;

ALTER TABLE osu_match_game_scores ADD COLUMN team_new int2;

UPDATE osu_match_game_scores
	SET team_new = CASE
		WHEN team = 'none' THEN 0 
		WHEN team = 'red' THEN 1 
		WHEN team = 'blue' THEN 2
		ELSE NULL
	END;

ALTER TABLE osu_match_game_scores DROP COLUMN team;
ALTER TABLE osu_match_game_scores RENAME COLUMN team_new TO team;
ALTER TABLE osu_match_game_scores ALTER COLUMN team SET NOT NULL;
