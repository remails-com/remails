INSERT INTO messages (id, smtp_credential_id, organization_id, status, from_email, recipients, raw_data, message_data)
VALUES ('e165562a-fb6d-423b-b318-fd26f4610634',
        '9442cbbf-9897-4af7-9766-4ac9c1bf49cf',
        '44729d9f-a7dc-4226-b412-36a7537f5176',
        'processing',
        'email@test-org-1.com',
        '{"info@recipient1.com", "info@recipient1.com"}',
        '',
        'null'::jsonb);
