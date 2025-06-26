INSERT INTO organizations (id, name, remaining_message_quota, quota_reset, remaining_rate_limit, rate_limit_reset)
VALUES ('44729d9f-a7dc-4226-b412-36a7537f5176',
        'test org 1',
        5000, now() + '1 month',
        500000, now() + '1 day'), -- practically unlimited rate limit for testing quota
       ('5d55aec5-136a-407c-952f-5348d4398204',
        'test org 2',
        500, now() + '1 month',
        500000, now() + '1 day')
