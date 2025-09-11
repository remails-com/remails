INSERT INTO organization_invites (id, organization_id, role, created_by, password_hash, expires_at)
VALUES ('32bba198-fdd8-4cb7-8b82-85857dd2527f',
        '44729d9f-a7dc-4226-b412-36a7537f5176', -- org 1
        'admin',
        '9244a050-7d72-451a-9248-4b43d5108235', -- test user 1
        '$argon2id$v=19$m=16,t=2,p=1$WWJQYkFxb0lXc0JkSHFDTw$4BYwZvcePs/l2WewT+TpcQ', -- password is unsecure
        now() + INTERVAL '7d'),
       ('516e1804-1d4b-44d4-b4ac-9d81a6b554e7',
        '44729d9f-a7dc-4226-b412-36a7537f5176', -- org 1
        'maintainer',
        '9244a050-7d72-451a-9248-4b43d5108235', -- test user 1
        '$argon2id$v=19$m=16,t=2,p=1$WWJQYkFxb0lXc0JkSHFDTw$4BYwZvcePs/l2WewT+TpcQ', -- password is unsecure
        now() + INTERVAL '7d'),
       ('dbbddca4-1e50-42bb-ac6e-6e8034ba666b',
        '44729d9f-a7dc-4226-b412-36a7537f5176', -- org 1
        'read_only',
        '9244a050-7d72-451a-9248-4b43d5108235', -- test user 1
        '$argon2id$v=19$m=16,t=2,p=1$WWJQYkFxb0lXc0JkSHFDTw$4BYwZvcePs/l2WewT+TpcQ', -- password is unsecure
        now() + INTERVAL '7d'),
       ('8b01ce56-4304-47c7-b9a6-62bd1b7e8269',
        '44729d9f-a7dc-4226-b412-36a7537f5176', -- org 1
        'admin',
        '9244a050-7d72-451a-9248-4b43d5108235', -- test user 1
        '$argon2id$v=19$m=16,t=2,p=1$WWJQYkFxb0lXc0JkSHFDTw$4BYwZvcePs/l2WewT+TpcQ', -- password is unsecure
        now() - INTERVAL '1d'); -- expired invite
