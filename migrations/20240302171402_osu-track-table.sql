-- Add migration script here
CREATE TABLE discord_channels (
  channel_id bigint PRIMARY KEY
);

CREATE TABLE "osu_tracking" (
  osu_id bigint,
  channel_id bigint,
  constraint fk_channel_id foreign KEY("channel_id") REFERENCES discord_channels(channel_id)
);
