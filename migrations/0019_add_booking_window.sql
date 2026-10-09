-- How far ahead a visitor can book. The notice period and the pause switch
-- already have columns: `minimum_notice_hours` and `is_active`.
ALTER TABLE widget_settings
    ADD COLUMN booking_window_days INT NOT NULL DEFAULT 7,
    ADD CONSTRAINT widget_booking_window CHECK (booking_window_days BETWEEN 1 AND 90);
