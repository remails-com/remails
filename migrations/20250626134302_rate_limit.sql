ALTER TABLE organizations
ADD remaining_rate_limit bigint                   NOT NULL DEFAULT 0,
ADD rate_limit_reset     timestamp with time zone NOT NULL DEFAULT now();