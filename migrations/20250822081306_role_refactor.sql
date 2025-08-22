-- We will have to manually reset the global admin roles after this migration
DROP TABLE api_users_global_roles;

ALTER TABLE api_users ADD global_role role;
