ALTER TABLE organizations
    ADD current_subscription jsonb NOT NULL DEFAULT '{
      "status": "none"
    }';

ALTER TABLE organizations
    ALTER COLUMN quota_reset DROP NOT NULL;