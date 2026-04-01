ALTER TABLE api_users ADD blocked boolean NOT NULL DEFAULT false;

ALTER TYPE org_block_status ADD VALUE 'full_freeze';

ALTER TYPE org_role ADD ATTRIBUTE org_block_status org_block_status;
