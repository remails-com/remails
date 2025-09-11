ALTER TYPE role ADD VALUE 'maintainer';
ALTER TYPE role ADD VALUE 'read_only';

ALTER TABLE organization_invites ADD role role NOT NULL;
