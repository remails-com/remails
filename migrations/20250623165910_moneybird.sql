ALTER TABLE organizations
    ADD moneybird_contact_id varchar DEFAULT NULL;

ALTER TABLE organizations
    ADD subscription_product varchar NOT NULL DEFAULT 'RmlsFree';

CREATE TABLE moneybird_webhook
(
    moneybird_id varchar NOT NULL PRIMARY KEY,
    token        varchar NOT NULL
);