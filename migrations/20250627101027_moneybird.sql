ALTER TABLE organizations
    ADD moneybird_contact_id varchar DEFAULT NULL,
    ADD total_message_quota  bigint NOT NULL DEFAULT 0,
    ADD used_message_quota   bigint NOT NULL DEFAULT 0,
    DROP COLUMN remaining_message_quota;

CREATE TABLE moneybird_webhook
(
    moneybird_id varchar NOT NULL PRIMARY KEY,
    token        varchar NOT NULL
);