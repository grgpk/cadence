-- A call keeps the length it was booked with. Until now only its start was
-- stored and the length was read from whatever hours covered that start, so
-- editing hours or the host's timezone changed calls that already existed.
ALTER TABLE bookings ADD COLUMN slot_end TIMESTAMPTZ;

-- Existing calls take the length of the hours that open their start today,
-- read on the host's clock. `weekday` is 0 for Monday. Where two sets of
-- hours open the same start, the earlier one wins, as it does when slots are
-- listed. A timezone Postgres does not know opens nothing.
WITH zones AS MATERIALIZED (
    SELECT host_unid, timezone FROM widget_settings
    WHERE timezone IN (SELECT name FROM pg_timezone_names)
),
local_starts AS (
    SELECT b.unid, b.host_unid, b.slot_start AT TIME ZONE z.timezone AS local_start
    FROM bookings b JOIN zones z ON z.host_unid = b.host_unid
),
lengths AS (
    SELECT DISTINCT ON (l.unid) l.unid, a.slot_minutes
    FROM local_starts l
    JOIN availability a
      ON a.host_unid = l.host_unid
     AND a.weekday = EXTRACT(ISODOW FROM l.local_start)::int - 1
     AND l.local_start::time >= a.start_time
     -- The whole call fits before the hours end.
     AND l.local_start::time - a.start_time + make_interval(mins => a.slot_minutes)
         <= a.end_time - a.start_time
     -- The start is one of the steps the hours open.
     AND mod(EXTRACT(EPOCH FROM l.local_start::time - a.start_time), a.slot_minutes * 60) = 0
    ORDER BY l.unid, a.start_time, a.id
)
UPDATE bookings b
SET slot_end = b.slot_start + make_interval(mins => l.slot_minutes)
FROM lengths l
WHERE l.unid = b.unid;

-- Calls whose hours are gone were shown as 30 minutes long.
UPDATE bookings SET slot_end = slot_start + interval '30 minutes' WHERE slot_end IS NULL;

ALTER TABLE bookings
    ALTER COLUMN slot_end SET NOT NULL,
    ADD CONSTRAINT bookings_slot_order CHECK (slot_end > slot_start);
