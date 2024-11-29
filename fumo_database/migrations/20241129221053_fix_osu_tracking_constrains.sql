-- Add migration script here

ALTER TABLE osu_tracking RENAME TO osu_tracking_temp;

CREATE TABLE osu_tracking (
  osu_id bigint not null,
  channel_id bigint NOT NULL,
  constraint fk_channel_id foreign KEY("channel_id") REFERENCES discord_channels(channel_id)
);

alter table osu_tracking add constraint unique_osu_channel UNIQUE(osu_id, channel_id);

INSERT INTO osu_tracking (osu_id, channel_id)
SELECT osu_id, channel_id 
FROM osu_tracking_temp
ON CONFLICT (osu_id, channel_id) DO NOTHING;

DROP TABLE osu_tracking_temp;
