-- Development seed users and role access grants.
-- Login password for all seeded users: password

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
        now(),
        'Root',
        '00000000-0000-4000-8000-100000000001'
    ),
    (
        'a1000000-0000-4000-8000-000000000002',
        now(),
        'Admin',
        '00000000-0000-4000-8000-100000000002'
    ),
    (
        'a1000000-0000-4000-8000-000000000003',
        now(),
        'Host',
        '00000000-0000-4000-8000-100000000003'
    )
ON CONFLICT DO NOTHING;
