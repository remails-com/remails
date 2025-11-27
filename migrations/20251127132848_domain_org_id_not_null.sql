ALTER TABLE domains DROP CONSTRAINT either_organization_or_project;

-- fill in organization IDs where missing
UPDATE domains SET organization_id=(SELECT organization_id FROM projects WHERE projects.id = domains.project_id) WHERE organization_id IS NULL;

ALTER TABLE domains ALTER COLUMN organization_id SET NOT NULL;
