-- A host's own place to meet (Zoom, a personal Meet room, any https link),
-- used when Google Calendar does not create a Meet link for the call.
ALTER TABLE widget_settings ADD COLUMN meeting_link TEXT;
