CREATE TYPE org_block_status AS ENUM (
    'not_blocked',
    'no_sending',
    'no_sending_or_receiving'
);

ALTER TABLE organizations ADD block_status org_block_status NOT NULL DEFAULT 'not_blocked';
