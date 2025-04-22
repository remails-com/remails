INSERT INTO api_users (id, email, name)
VALUES ('9244a050-7d72-451a-9248-4b43d5108235', 'test-api-user-1', 'Test API User 1');

INSERT INTO api_users_organizations (api_user_id, organization_id, role)
VALUES ('9244a050-7d72-451a-9248-4b43d5108235', '44729d9f-a7dc-4226-b412-36a7537f5176', 'admin');

INSERT INTO api_users (id, email, name)
VALUES ('94a98d6f-1ec0-49d2-a951-92dc0ff3042a', 'test-api@user-2', 'Test API User 2');

INSERT INTO api_users_organizations (api_user_id, organization_id, role)
VALUES ('94a98d6f-1ec0-49d2-a951-92dc0ff3042a', '5d55aec5-136a-407c-952f-5348d4398204', 'admin');