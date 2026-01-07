ALTER TABLE domains
    ADD COLUMN verification_status    jsonb       NOT NULL DEFAULT '{
      "timestamp": "2025-12-22T08:13:21.532824093Z",
      "dkim": {
        "status": "Success",
        "reason": "available!",
        "value": null
      },
      "spf": {
        "status": "Success",
        "reason": "correct!",
        "value": null
      },
      "dmarc": {
        "status": "Success",
        "reason": "correct!",
        "value": null
      },
      "a": {
        "status": "Success",
        "reason": "available",
        "value": null
      }
    }',
    ADD COLUMN last_verification_time timestamptz NOT NULL DEFAULT '2025-12-22T08:13:21.532824093Z';

CREATE INDEX domain_verification_time ON domains (last_verification_time);

ALTER TABLE domains
    ALTER COLUMN verification_status DROP DEFAULT,
    ALTER COLUMN last_verification_time DROP DEFAULT;
