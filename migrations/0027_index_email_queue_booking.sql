-- Emails are dropped and replaced per call, and deleting a call cascades to
-- them; both looked through the whole queue. The scan for due emails already
-- has `email_queue_pending_idx`.
CREATE INDEX email_queue_booking_idx ON email_queue (booking_unid);
