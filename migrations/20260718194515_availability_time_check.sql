-- Add migration script here
DELETE FROM availability WHERE start_time >= end_time;
ALTER TABLE availability
    ADD CONSTRAINT availability_time_order CHECK (start_time < end_time);