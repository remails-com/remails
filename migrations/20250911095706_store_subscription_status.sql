ALTER TABLE organizations
    ADD current_subscription jsonb NOT NULL DEFAULT '{
      "status": "none"
    }';

ALTER TABLE organizations
    ALTER COLUMN quota_reset DROP NOT NULL;

CREATE UNIQUE INDEX organizations_moneybird_contact_id
    ON organizations (moneybird_contact_id);
