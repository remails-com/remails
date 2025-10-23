INSERT INTO api_keys (id, description, password_hash, organization_id, role)
VALUES ('951ec618-bcc9-4224-9cf1-ed41a84f41d8',
        'Test API key unknown password',
        '$argon2id$v=19$m=16,t=2,p=1$VzVENmtXRXFzaU5hTHJxQg$zErqgE1EMeHP21UbXSaLNA',
        '44729d9f-a7dc-4226-b412-36a7537f5176', -- Organization 1
        'maintainer');
