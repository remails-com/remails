INSERT INTO smtp_credentials (id, project_id, username, password_hash, description)
VALUES ('9442cbbf-9897-4af7-9766-4ac9c1bf49cf',
           -- Stream 1 Project 1 Organization 1
        '3ba14adf-4de1-4fb6-8c20-50cc2ded5462',
        'marc',
           -- PW: check 1Password
        '$argon2id$v=19$m=16,t=2,p=1$VzVENmtXRXFzaU5hTHJxQg$zErqgE1EMeHP21UbXSaLNA',
        'Test SMTP credential');

INSERT INTO smtp_credentials (id, project_id, username, password_hash, description)
VALUES ('abbb0388-bdfa-4758-8ad0-80035999ab6c',
           -- Stream 1 Project 2 Organization 1
        'da12d059-d86e-4ac6-803d-d013045f68ff',
        'foo',
           -- we don't know this
        '$argon2id$v=19$m=16,t=2,p=1$SXlxN0U3VXNnSXN6UENWeA$2wsKyY0Ikz1qyeiWLO8SWg',
        'Test SMTP credential 2');