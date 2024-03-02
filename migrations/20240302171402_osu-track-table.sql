-- Add migration script here
CREATE TABLE discord_channels (
  channel_id bigint PRIMARY KEY NOT NULL
);

CREATE TABLE "osu_tracking" (
  osu_id bigint not null,
  channel_id bigint NOT NULL,
  constraint fk_channel_id foreign KEY("channel_id") REFERENCES discord_channels(channel_id)
);
