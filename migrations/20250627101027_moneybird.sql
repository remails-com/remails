ALTER TABLE organizations
    ADD moneybird_contact_id varchar DEFAULT NULL,
    -- Currently, the product in the DB is only used to calculate the quota difference when the subscription changes
    ADD subscription_product varchar NOT NULL DEFAULT 'RmlsFree';

CREATE TABLE moneybird_webhook
(
    moneybird_id varchar NOT NULL PRIMARY KEY,
    token        varchar NOT NULL
);