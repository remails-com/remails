ALTER TABLE messages
ADD COLUMN quota_charged boolean NOT NULL DEFAULT false;

UPDATE messages
SET quota_charged = true
WHERE status IN ('accepted', 'reattempt', 'failed');
