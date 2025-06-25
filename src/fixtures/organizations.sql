INSERT INTO organizations (id, name, remaining_message_quota, quota_reset)
VALUES ('44729d9f-a7dc-4226-b412-36a7537f5176', 'test org 1', 5000, now() + INTERVAL '1 month'),
       ('5d55aec5-136a-407c-952f-5348d4398204', 'test org 2', 500, now() + INTERVAL '1 month'),
       ('533d9a19-16e8-4a1b-a824-ff50af8b428c', 'quota reset test org 1', 500, now() - INTERVAL '1 minute'),
       ('ee14cdb8-f62e-42ac-a0cd-294d708be994', 'quota reset test org 2', 500,
        '2025-06-25 23:59:59.000000 +00:00'::timestamp with time zone - INTERVAL '100 months'),
       ('7b2d91d0-f9d9-4ddd-88ac-6853f736501c', 'quota reset test org 3', 333, now() + INTERVAL '1 minute'),
       ('0f83bfee-e7b6-4670-83ec-192afec2b137', 'quota reset test org 4', 333, '2025-01-31 23:59:59.000000 +00:00')
