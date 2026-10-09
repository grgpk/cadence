-- A cancelled call gives its time back: only calls that still hold their
-- slot have to be unique per host.
ALTER TABLE bookings DROP CONSTRAINT bookings_slot_unique;

CREATE UNIQUE INDEX bookings_slot_unique
    ON bookings (host_unid, slot_start)
    WHERE status <> 'cancelled';
