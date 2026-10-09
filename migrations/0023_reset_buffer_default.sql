-- `buffer_minutes` was never read, so its default of 15 meant nothing. Now
-- that it closes times next to a booked call, every host starts at 0 and
-- nobody's open times change until they choose a buffer.
UPDATE widget_settings SET buffer_minutes = 0;

ALTER TABLE widget_settings ALTER COLUMN buffer_minutes SET DEFAULT 0;
