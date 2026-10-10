-- The development seed used to run as a migration, so production got its
-- three logins with a published password. This switches off each one that
-- still has that password. Nothing is deleted, and an account whose password
-- was changed is left alone.
UPDATE users SET status = 'Banned'
WHERE unid IN (
        '00000000-0000-4000-8000-100000000001',
        '00000000-0000-4000-8000-100000000002',
        '00000000-0000-4000-8000-100000000003'
    )
  AND password_hash = '$argon2id$v=19$m=19456,t=2,p=1$DjUgDuDjZ8aQm6zZpyn9Fw$qHOAG91yV3/6OhDFYYI0nFMJnkxtrzO6txtkqMraKFo';
