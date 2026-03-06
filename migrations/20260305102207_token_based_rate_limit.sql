ALTER TABLE organizations
    DROP COLUMN rate_limit_reset,
    DROP COLUMN remaining_rate_limit,
    ADD COLUMN rate_limit_tokens    BIGINT                   NOT NULL DEFAULT 0,
    ADD COLUMN rate_limit_last_used timestamp with time zone NOT NULL DEFAULT now();
