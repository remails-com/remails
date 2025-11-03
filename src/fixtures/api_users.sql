INSERT INTO api_users (id, email, name, password_hash) -- user 1: admin of org 1 and org 2
VALUES ('9244a050-7d72-451a-9248-4b43d5108235', 'admin@example.com', 'Test API User 1',
        '$argon2id$v=19$m=16,t=2,p=1$TEVEQWk2eGJMRDJZalZJbg$VUjsIMHx9udxdHJq/vHRUQ');
INSERT INTO api_users_organizations (api_user_id, organization_id, role)
VALUES ('9244a050-7d72-451a-9248-4b43d5108235', '44729d9f-a7dc-4226-b412-36a7537f5176', 'admin'),
       ('9244a050-7d72-451a-9248-4b43d5108235', '5d55aec5-136a-407c-952f-5348d4398204', 'admin');

INSERT INTO api_users (id, email, name) -- user 2: admin of org 2
VALUES ('94a98d6f-1ec0-49d2-a951-92dc0ff3042a', 'test-api@user-2', 'Test API User 2');
INSERT INTO api_users_organizations (api_user_id, organization_id, role)
VALUES ('94a98d6f-1ec0-49d2-a951-92dc0ff3042a', '5d55aec5-136a-407c-952f-5348d4398204', 'admin');

INSERT INTO api_users (id, email, name, password_hash) -- user 3: not in any organization, password is unsecure123
VALUES ('54432300-128a-46a0-8a83-fe39ce3ce5ef', 'test-api@user-3', 'Test API User 3',
        '$argon2id$v=19$m=16,t=2,p=1$VlZ0SUUzdXRBZVFldEZpbQ$rNhHR3o94Zw1B4YVqty6xQ');

INSERT INTO api_users (id, email, name) -- user 4: maintainer of org 1
VALUES ('c33dbd88-43ed-404b-9367-1659a73c8f3a', 'test-api@user-4', 'Test API User 4');
INSERT INTO api_users_organizations (api_user_id, organization_id, role)
VALUES ('c33dbd88-43ed-404b-9367-1659a73c8f3a', '44729d9f-a7dc-4226-b412-36a7537f5176', 'maintainer');

INSERT INTO api_users (id, email, name) -- user 5: read only in org 1
VALUES ('703bf1cb-7a3e-4640-83bf-1b07ce18cd2e', 'test-api@user-5', 'Test API User 5');
INSERT INTO api_users_organizations (api_user_id, organization_id, role)
VALUES ('703bf1cb-7a3e-4640-83bf-1b07ce18cd2e', '44729d9f-a7dc-4226-b412-36a7537f5176', 'read_only');

INSERT INTO api_users (id, email, name, global_role) -- super admin
VALUES ('deadbeef-4e43-4a66-bbb9-fbcd4a933a34', 'sudo@remails', 'Super Admin', 'admin');

INSERT INTO api_users (id, email, name, password_hash) -- not in any organization, password is unsecure123
VALUES ('820128b1-e08f-404d-ad08-e679a7d6b515', 'test-totp@user-4', 'TOTP API User user 4',
        '$argon2id$v=19$m=16,t=2,p=1$VlZ0SUUzdXRBZVFldEZpbQ$rNhHR3o94Zw1B4YVqty6xQ');

INSERT INTO api_users (id, email, name, password_hash) -- not in any organization, password is unsecure123
VALUES ('672be18f-a89e-4a1d-adaa-45a0b4e2f350', 'test-totp-rate-limit@user-4',
        'TOTP API User user for rate limit testing',
        '$argon2id$v=19$m=16,t=2,p=1$VlZ0SUUzdXRBZVFldEZpbQ$rNhHR3o94Zw1B4YVqty6xQ');
INSERT INTO totp (id, description, user_id, url, state, last_used)
VALUES ('448f8b7c-e6b9-4038-ab73-bc35826fd5da', '', '672be18f-a89e-4a1d-adaa-45a0b4e2f350',
        'otpauth://totp/Remails:test-totp-rate-limit%40user-4?secret=CP32OBJWEI6FDV3Z7UDMAQT5YDYUS36L&algorithm=SHA256&issuer=Remails',
        'enabled', null);

INSERT INTO api_users (id, email, name) -- Read-only member of the "webhook test" organization
VALUES ('d57373be-cb77-4a2b-9e6e-66b28c4b5c7e', 'webhook@test.com', 'Webhook test');
INSERT INTO api_users_organizations (api_user_id, organization_id, role)
VALUES ('d57373be-cb77-4a2b-9e6e-66b28c4b5c7e', 'ad76a517-3ff2-4d84-8299-742847782d4d', 'read_only');


INSERT INTO api_users (id, email, name) -- Read-only member of the "First subscription become admin test with two members" organization
VALUES ('6bd0c3a0-7053-4bb1-b20a-3c144373fe30', 'subscription@test.com', 'subscription test');
INSERT INTO api_users_organizations (api_user_id, organization_id, role)
VALUES ('6bd0c3a0-7053-4bb1-b20a-3c144373fe30', 'e11df9da-56f5-433c-9d3a-dd338f262c66', 'read_only');

INSERT INTO api_users (id, email, name) -- Read-only member of the "First subscription become admin test with two members" organization
VALUES ('9aa7656f-6f52-4a4c-b7cf-93600e613177', 'subscription2@test.com', 'subscription2 test');
INSERT INTO api_users_organizations (api_user_id, organization_id, role)
VALUES ('9aa7656f-6f52-4a4c-b7cf-93600e613177', 'e11df9da-56f5-433c-9d3a-dd338f262c66', 'read_only');