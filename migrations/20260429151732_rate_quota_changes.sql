ALTER TABLE messages
ADD COLUMN quota_charged boolean NOT NULL DEFAULT false;

UPDATE messages
SET quota_charged = true
WHERE status IN ('accepted', 'reattempt', 'failed');

ALTER TABLE messages
ADD COLUMN check_attempts integer NOT NULL DEFAULT 0,
ADD COLUMN delivery_attempts integer NOT NULL DEFAULT 0,
ADD COLUMN max_check_attempts integer NOT NULL DEFAULT 0,
ADD COLUMN max_delivery_attempts integer NOT NULL DEFAULT 0;

-- For messages that never passed the check phase, all attempts were check attempts
UPDATE messages SET
    check_attempts = attempts,
    max_check_attempts = max_attempts,
    max_delivery_attempts = max_attempts
WHERE status IN ('processing', 'held', 'rejected');

-- For messages that reached delivery, credit 1 check attempt and the rest as delivery
UPDATE messages SET
    check_attempts = 1,
    delivery_attempts = GREATEST(0, attempts - 1),
    max_check_attempts = max_attempts,
    max_delivery_attempts = max_attempts
WHERE status IN ('accepted', 'reattempt', 'failed', 'delivered');

ALTER TABLE messages
DROP COLUMN attempts,
DROP COLUMN max_attempts;
