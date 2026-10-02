-- Development seed data for the complete Cadence flow.
-- Login password for all seeded users: password
-- IDs stay fixed so local resets produce stable API fixtures.

-- Users and access grants.
INSERT INTO users (unid, email, password_hash, full_name, status) VALUES
    (
        '00000000-0000-4000-8000-100000000001',
        'root@example.com',
        '$argon2id$v=19$m=19456,t=2,p=1$DjUgDuDjZ8aQm6zZpyn9Fw$qHOAG91yV3/6OhDFYYI0nFMJnkxtrzO6txtkqMraKFo',
        'Cadence Root',
        'Active'
    ),
    (
        '00000000-0000-4000-8000-100000000002',
        'admin@example.com',
        '$argon2id$v=19$m=19456,t=2,p=1$DjUgDuDjZ8aQm6zZpyn9Fw$qHOAG91yV3/6OhDFYYI0nFMJnkxtrzO6txtkqMraKFo',
        'Cadence Admin',
        'Active'
    ),
    (
        '00000000-0000-4000-8000-100000000003',
        'host@example.com',
        '$argon2id$v=19$m=19456,t=2,p=1$DjUgDuDjZ8aQm6zZpyn9Fw$qHOAG91yV3/6OhDFYYI0nFMJnkxtrzO6txtkqMraKFo',
        'Cadence Host',
        'Active'
    )
ON CONFLICT DO NOTHING;

INSERT INTO roleaccesses (unid, created_at, role, grantedto_unid) VALUES
    (
        'a1000000-0000-4000-8000-000000000001',
        now() - interval '30 days',
        'Root',
        '00000000-0000-4000-8000-100000000001'
    ),
    (
        'a1000000-0000-4000-8000-000000000002',
        now() - interval '29 days',
        'Admin',
        '00000000-0000-4000-8000-100000000002'
    ),
    (
        'a1000000-0000-4000-8000-000000000003',
        now() - interval '28 days',
        'Host',
        '00000000-0000-4000-8000-100000000003'
    )
ON CONFLICT DO NOTHING;

-- Public booking widget configuration.
INSERT INTO widget_settings (
    host_unid,
    public_slug,
    title,
    description,
    accent_color,
    duration_minutes,
    buffer_minutes,
    minimum_notice_hours,
    timezone,
    is_active,
    created_at,
    updated_at
) VALUES (
    '00000000-0000-4000-8000-100000000003',
    'cadence-host',
    'Book a coaching call',
    'Choose a focused 30-minute conversation about your next career move.',
    '#7c3aed',
    30,
    15,
    4,
    'Europe/Paris',
    true,
    now() - interval '21 days',
    now() - interval '2 hours'
)
ON CONFLICT DO NOTHING;

-- Monday through Sunday windows. Service weekday numbering: Monday = 0.
INSERT INTO availability (
    id,
    host_unid,
    weekday,
    start_time,
    end_time,
    slot_minutes,
    created_at
) VALUES
    (1001, '00000000-0000-4000-8000-100000000003', 0, '09:00', '17:00', 30, now() - interval '20 days'),
    (1002, '00000000-0000-4000-8000-100000000003', 1, '09:00', '17:00', 30, now() - interval '20 days'),
    (1003, '00000000-0000-4000-8000-100000000003', 2, '09:00', '17:00', 30, now() - interval '20 days'),
    (1004, '00000000-0000-4000-8000-100000000003', 3, '09:00', '17:00', 30, now() - interval '20 days'),
    (1005, '00000000-0000-4000-8000-100000000003', 4, '09:00', '17:00', 30, now() - interval '20 days'),
    (1006, '00000000-0000-4000-8000-100000000003', 5, '10:00', '14:00', 30, now() - interval '20 days'),
    (1007, '00000000-0000-4000-8000-100000000003', 6, '10:00', '14:00', 30, now() - interval '20 days')
ON CONFLICT DO NOTHING;

-- Funnel visits. Some visitors abandon, some submit, and some book.
INSERT INTO calling_visits (
    unid,
    source_page,
    referer,
    ip_address,
    user_agent,
    accept_language,
    is_mobile,
    country,
    city,
    utm_source,
    utm_medium,
    utm_campaign,
    duration_seconds,
    form_started,
    form_submitted,
    booked_call,
    created_at,
    updated_at
) VALUES
    (
        '20000000-0000-4000-8000-000000000001',
        '/book-a-call',
        'https://www.google.com/',
        '203.0.113.10',
        'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)',
        'en-US,en;q=0.9',
        false,
        'United States',
        'New York',
        'google',
        'organic',
        'career-coaching',
        842,
        true,
        true,
        true,
        now() - interval '3 days',
        now() - interval '3 days'
    ),
    (
        '20000000-0000-4000-8000-000000000002',
        '/book-a-call',
        'https://www.linkedin.com/',
        '203.0.113.11',
        'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)',
        'en-GB,en;q=0.8',
        true,
        'United Kingdom',
        'London',
        'linkedin',
        'social',
        'october-outreach',
        516,
        true,
        true,
        false,
        now() - interval '2 days',
        now() - interval '2 days'
    ),
    (
        '20000000-0000-4000-8000-000000000003',
        '/book-a-call',
        'https://newsletter.example.com/',
        '203.0.113.12',
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64)',
        'fr-FR,fr;q=0.9',
        false,
        'France',
        'Paris',
        'newsletter',
        'email',
        'career-reset',
        74,
        true,
        false,
        false,
        now() - interval '18 hours',
        now() - interval '18 hours'
    ),
    (
        '20000000-0000-4000-8000-000000000004',
        '/',
        NULL,
        '203.0.113.13',
        'Mozilla/5.0 (Linux; Android 14; Pixel 8)',
        'de-DE,de;q=0.9',
        true,
        'Germany',
        'Berlin',
        NULL,
        NULL,
        NULL,
        12,
        false,
        false,
        false,
        now() - interval '6 hours',
        now() - interval '6 hours'
    )
ON CONFLICT DO NOTHING;

-- Lead form records in each qualification state.
INSERT INTO leads (
    unid,
    host_unid,
    created_at,
    updated_at,
    first_name,
    last_name,
    email,
    phone,
    country_code,
    investment_comfort,
    what_stopping_you,
    source_page,
    utm_source,
    utm_medium,
    utm_campaign,
    qualification_status,
    form_submitted_at,
    ip_address,
    user_agent
) VALUES
    (
        '30000000-0000-4000-8000-000000000001',
        '00000000-0000-4000-8000-100000000003',
        now() - interval '3 days',
        now() - interval '3 days',
        'Maya',
        'Chen',
        'maya.chen@example.com',
        '+1 212 555 0142',
        'US',
        'Comfortable investing in focused support',
        'I have interviews but struggle to explain my impact clearly.',
        '/book-a-call',
        'google',
        'organic',
        'career-coaching',
        'Qualified',
        now() - interval '3 days',
        '203.0.113.10',
        'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)'
    ),
    (
        '30000000-0000-4000-8000-000000000002',
        '00000000-0000-4000-8000-100000000003',
        now() - interval '2 days',
        now() - interval '2 days',
        'Oliver',
        'Grant',
        'oliver.grant@example.com',
        '+44 20 7946 0958',
        'GB',
        'I need to understand the options first',
        'I keep applying to roles without getting interviews.',
        '/book-a-call',
        'linkedin',
        'social',
        'october-outreach',
        'NotSure',
        now() - interval '2 days',
        '203.0.113.11',
        'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)'
    ),
    (
        '30000000-0000-4000-8000-000000000003',
        '00000000-0000-4000-8000-100000000003',
        now() - interval '10 days',
        now() - interval '9 days',
        'Sofia',
        'Martin',
        'sofia.martin@example.com',
        NULL,
        'FR',
        'Not ready to invest right now',
        'I am still exploring whether I want to change careers.',
        '/book-a-call',
        'newsletter',
        'email',
        'career-reset',
        'Disqualified',
        now() - interval '10 days',
        '203.0.113.12',
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64)'
    ),
    (
        '30000000-0000-4000-8000-000000000004',
        '00000000-0000-4000-8000-100000000003',
        now() - interval '6 hours',
        now() - interval '6 hours',
        'Noah',
        'Kowalski',
        'noah.kowalski@example.com',
        '+49 30 5550 1832',
        'DE',
        'I want to compare coaching options',
        'I am unsure how to position a move from engineering into leadership.',
        '/',
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        '203.0.113.13',
        'Mozilla/5.0 (Linux; Android 14; Pixel 8)'
    )
ON CONFLICT DO NOTHING;

-- Bookings include confirmed, pending, and cancelled states.
-- Slot timestamps remain relative so the local widget always has future fixtures.
INSERT INTO bookings (
    unid,
    host_unid,
    lead_unid,
    calling_visit_unid,
    slot_start,
    timezone,
    invitee_name,
    invitee_email,
    status,
    google_event_id,
    google_meet_url,
    cancelled_at,
    created_at,
    updated_at
) VALUES
    (
        '40000000-0000-4000-8000-000000000001',
        '00000000-0000-4000-8000-100000000003',
        '30000000-0000-4000-8000-000000000001',
        '20000000-0000-4000-8000-000000000001',
        date_trunc('day', now()) + interval '2 days 10 hours',
        'America/New_York',
        'Maya Chen',
        'maya.chen@example.com',
        'confirmed',
        'cadence-demo-event-001',
        'https://meet.google.com/demo-cadence-001',
        NULL,
        now() - interval '3 days',
        now() - interval '3 days'
    ),
    (
        '40000000-0000-4000-8000-000000000002',
        '00000000-0000-4000-8000-100000000003',
        '30000000-0000-4000-8000-000000000002',
        '20000000-0000-4000-8000-000000000002',
        date_trunc('day', now()) + interval '4 days 14 hours',
        'Europe/London',
        'Oliver Grant',
        'oliver.grant@example.com',
        'pending',
        NULL,
        NULL,
        NULL,
        now() - interval '2 days',
        now() - interval '2 days'
    ),
    (
        '40000000-0000-4000-8000-000000000003',
        '00000000-0000-4000-8000-100000000003',
        '30000000-0000-4000-8000-000000000003',
        NULL,
        date_trunc('day', now()) + interval '6 days 11 hours',
        'Europe/Paris',
        'Sofia Martin',
        'sofia.martin@example.com',
        'cancelled',
        'cadence-demo-event-003',
        NULL,
        now() - interval '8 days',
        now() - interval '10 days',
        now() - interval '8 days'
    )
ON CONFLICT DO NOTHING;

-- Queued booking mail: pending, sent, and failed worker states.
INSERT INTO email_queue (
    id,
    booking_unid,
    email,
    template,
    send_at,
    sent_at,
    attempts,
    last_error,
    created_at,
    payload
) VALUES
    (
        '50000000-0000-4000-8000-000000000001',
        '40000000-0000-4000-8000-000000000001',
        'delivered@resend.dev',
        'booking_confirmation',
        now() - interval '3 days',
        now() - interval '3 days',
        0,
        NULL,
        now() - interval '3 days',
        jsonb_build_object(
            'name', 'Maya Chen',
            'slot_start', date_trunc('day', now()) + interval '2 days 10 hours',
            'timezone', 'America/New_York'
        )
    ),
    (
        '50000000-0000-4000-8000-000000000002',
        '40000000-0000-4000-8000-000000000001',
        'delivered@resend.dev',
        'reminder_24h',
        date_trunc('day', now()) + interval '1 day 10 hours',
        NULL,
        0,
        NULL,
        now() - interval '3 days',
        jsonb_build_object(
            'name', 'Maya Chen',
            'slot_start', date_trunc('day', now()) + interval '2 days 10 hours',
            'timezone', 'America/New_York'
        )
    ),
    (
        '50000000-0000-4000-8000-000000000003',
        '40000000-0000-4000-8000-000000000002',
        'delivered@resend.dev',
        'booking_confirmation',
        now() - interval '2 days',
        NULL,
        1,
        'SMTP connection unavailable in local development',
        now() - interval '2 days',
        jsonb_build_object(
            'name', 'Oliver Grant',
            'slot_start', date_trunc('day', now()) + interval '4 days 14 hours',
            'timezone', 'Europe/London'
        )
    )
ON CONFLICT DO NOTHING;

-- Calendar integration fixtures. Tokens are deliberately fake local-dev values.
INSERT INTO google_calendar_config (
    host_unid,
    access_token,
    refresh_token,
    expires_at,
    calendar_id,
    updated_at
) VALUES (
    '00000000-0000-4000-8000-100000000003',
    'dev-access-token-not-real',
    'dev-refresh-token-not-real',
    now() + interval '1 hour',
    'primary',
    now() - interval '15 minutes'
)
ON CONFLICT DO NOTHING;

INSERT INTO oauth_states (
    state,
    user_unid,
    provider,
    redirect_uri,
    expires_at,
    consumed_at,
    created_at
) VALUES
    (
        'dev-oauth-state-pending',
        '00000000-0000-4000-8000-100000000003',
        'google',
        'http://localhost:3001/api/calendar/callback',
        now() + interval '10 minutes',
        NULL,
        now() - interval '5 minutes'
    ),
    (
        'dev-oauth-state-consumed',
        '00000000-0000-4000-8000-100000000003',
        'google',
        'http://localhost:3001/api/calendar/callback',
        now() - interval '2 days',
        now() - interval '2 days 1 minute',
        now() - interval '2 days 10 minutes'
    )
ON CONFLICT DO NOTHING;

-- Bug monitor fixtures. Two rows share a hash so admin aggregation is visible.
INSERT INTO bug_reports (
    id,
    unid,
    bugtype,
    similarityhash,
    message,
    exceptionmessage,
    stacktrace,
    userlogin,
    url,
    useragent,
    application,
    created
) VALUES
    (
        1001,
        '60000000-0000-4000-8000-000000000001',
        'JsError',
        610001,
        'TypeError: Cannot read properties of undefined (reading ''map'')',
        'TypeError: Cannot read properties of undefined (reading ''map'')',
        'at LeadsPage (leads.tsx:42:18)\nat renderWithHooks (react-dom.js:16305:18)',
        'admin@example.com',
        'http://localhost:3000/admin/leads',
        'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)',
        'cadence-local',
        now() - interval '4 days'
    ),
    (
        1002,
        '60000000-0000-4000-8000-000000000002',
        'JsError',
        610001,
        'TypeError: Cannot read properties of undefined (reading ''map'')',
        'TypeError: Cannot read properties of undefined (reading ''map'')',
        'at LeadsPage (leads.tsx:42:18)\nat renderWithHooks (react-dom.js:16305:18)',
        'host@example.com',
        'http://localhost:3000/dashboard',
        'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)',
        'cadence-local',
        now() - interval '3 days'
    ),
    (
        1003,
        '60000000-0000-4000-8000-000000000003',
        'Database',
        610002,
        'database connection pool exhausted',
        'database connection pool exhausted',
        'at sqlx::pool::Pool::acquire(pool.rs:410:9)',
        NULL,
        'http://localhost:3001/api/admin/bookings',
        'curl/8.5.0',
        'cadence-local',
        now() - interval '12 hours'
    )
ON CONFLICT DO NOTHING;

-- Audit trail fixtures for admin activity.
INSERT INTO audit_logs (
    unid,
    user_unid,
    action,
    resource_type,
    resource_unid,
    metadata,
    ip_address,
    created_at
) VALUES
    (
        '70000000-0000-4000-8000-000000000001',
        '00000000-0000-4000-8000-100000000002',
        'viewed',
        'leads',
        NULL,
        '{"path":"/admin/leads","result_count":4}'::jsonb,
        '127.0.0.1',
        now() - interval '2 hours'
    ),
    (
        '70000000-0000-4000-8000-000000000002',
        '00000000-0000-4000-8000-100000000003',
        'updated',
        'widget_settings',
        '00000000-0000-4000-8000-100000000003',
        '{"field":"accent_color","from":"#111111","to":"#7c3aed"}'::jsonb,
        '127.0.0.1',
        now() - interval '1 day'
    ),
    (
        '70000000-0000-4000-8000-000000000003',
        NULL,
        'created',
        'booking',
        '40000000-0000-4000-8000-000000000001',
        '{"source":"public_widget","invitee":"maya.chen@example.com"}'::jsonb,
        '203.0.113.10',
        now() - interval '3 days'
    )
ON CONFLICT DO NOTHING;

-- Rate-limit fixtures: one active window and one older window.
INSERT INTO rate_limits (action, subject, window_start, request_count) VALUES
    ('login', '127.0.0.1', date_trunc('minute', now()), 2),
    ('calling_visit', '203.0.113.10', date_trunc('minute', now()), 1),
    ('login', '198.51.100.20', date_trunc('minute', now() - interval '2 hours'), 5)
ON CONFLICT DO NOTHING;

-- Keep serial generators ahead of explicit fixture IDs for later local inserts.
SELECT setval(
    pg_get_serial_sequence('availability', 'id'),
    COALESCE((SELECT MAX(id) FROM availability), 1),
    true
);

SELECT setval(
    pg_get_serial_sequence('bug_reports', 'id'),
    COALESCE((SELECT MAX(id) FROM bug_reports), 1),
    true
);
