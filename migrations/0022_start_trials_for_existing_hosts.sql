-- Plan 37: hosts who signed up before plans existed have no subscription
-- row, which reads as Free. Rather than losing features the day plans ship,
-- they get the same 14-day Pro trial a new sign-up gets, from now, and the
-- usual trial-ending email 3 days before it ends. The worker fills in the
-- billing link and what they would lose when it sends.
WITH started AS (
    INSERT INTO subscriptions (user_unid, plan, status, trial_ends_at)
    SELECT DISTINCT r.grantedto_unid, 'pro', 'trialing', now() + interval '14 days'
    FROM roleaccesses r
    JOIN users u ON u.unid = r.grantedto_unid
    WHERE r.role = 'Host' AND u.status = 'Active'
    ON CONFLICT (user_unid) DO NOTHING
    RETURNING user_unid, trial_ends_at
)
INSERT INTO email_queue (email, template, send_at, payload)
SELECT u.email, 'trial_ending', s.trial_ends_at - interval '3 days',
       jsonb_build_object('name', u.full_name)
FROM started s
JOIN users u ON u.unid = s.user_unid;
