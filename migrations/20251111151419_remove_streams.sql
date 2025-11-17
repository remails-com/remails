ALTER TABLE smtp_credentials
    ADD project_id uuid REFERENCES projects (id) ON DELETE CASCADE;

UPDATE smtp_credentials
SET project_id = subquery.project_id
FROM (SELECT s.project_id, c.stream_id
      FROM smtp_credentials c
               JOIN streams s ON s.id = c.stream_id)
         AS subquery
WHERE subquery.stream_id = smtp_credentials.stream_id;

ALTER TABLE smtp_credentials
    ALTER COLUMN project_id SET NOT NULL,
    DROP COLUMN stream_id;

ALTER TABLE messages DROP COLUMN stream_id;

DROP TABLE streams;