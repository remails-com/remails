CREATE TABLE domains_projects (
    domain_id uuid NOT NULL REFERENCES domains (id) ON DELETE CASCADE,
    project_id uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    PRIMARY KEY (domain_id, project_id)
);

-- Insert not-null project ids
INSERT INTO domains_projects (domain_id, project_id)
SELECT id, project_id
FROM domains
WHERE project_id IS NOT NULL;

-- Insert all organization projects for domains with null project id 
INSERT INTO domains_projects (domain_id, project_id)
SELECT d.id, p.id
FROM domains d
JOIN projects p
ON p.organization_id = d.organization_id
WHERE d.project_id IS NULL;

-- Remove project id column
ALTER TABLE domains DROP COLUMN project_id;
