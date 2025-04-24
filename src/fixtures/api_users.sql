INSERT INTO api_users (id, email, name, password_hash)
VALUES ('9244a050-7d72-451a-9248-4b43d5108235', 'admin@example.com', 'Test API User 1', '$argon2id$v=19$m=19456,t=2,p=1$KjHW7NLUvyeNq0NmGjksoA$Z5ZDLWt4XMnNvDiRyzQenvJOzvutYWiwwiM9CxJ1iS8');

INSERT INTO api_users_organizations (api_user_id, organization_id, role)
VALUES ('9244a050-7d72-451a-9248-4b43d5108235', '44729d9f-a7dc-4226-b412-36a7537f5176', 'admin');

INSERT INTO api_users (id, email, name)
VALUES ('94a98d6f-1ec0-49d2-a951-92dc0ff3042a', 'test-api@user-2', 'Test API User 2');

INSERT INTO api_users_organizations (api_user_id, organization_id, role)
VALUES ('94a98d6f-1ec0-49d2-a951-92dc0ff3042a', '5d55aec5-136a-407c-952f-5348d4398204', 'admin');
