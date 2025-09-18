import { Alert, Button, Group, Select, Stack, Text, Tooltip } from "@mantine/core";
import { Organization, OrganizationMember, Role, User } from "../../types";
import { useState } from "react";
import { isValidRole, ROLE_INFO, roleSelectData } from "./NewInvite";
import { IconAlertTriangle } from "@tabler/icons-react";
import { ROLE_LABELS } from "../../util";

interface UpdateRoleProps {
  cancel: () => void;
  submit: (role: Role) => void;
  member: OrganizationMember;
  user: User;
  currentOrganization: Organization;
}

export default function UpdateRole({ cancel, submit, member, user, currentOrganization }: UpdateRoleProps) {
  const [role, setRole] = useState(member.role);

  if (!currentOrganization) {
    return null;
  }

  const role_unchanged = role == member.role;
  const role_name = ROLE_LABELS[role].toLowerCase();

  return (
    <Stack gap="xl">
      <Text>
        Please select which role the{" "}
        <Text span fw="bold">
          {member.name}
        </Text>{" "}
        should have:
      </Text>

      {ROLE_INFO}

      <Select
        data-autofocus
        label="Organization role"
        value={role}
        onChange={(value) => {
          if (value && isValidRole(value)) setRole(value);
        }}
        placeholder="Pick a role"
        data={roleSelectData}
      />

      {member.user_id == user.id && (
        <Alert icon={<IconAlertTriangle />}>
          <Text>
            You are editing{" "}
            <Text span fw="bold">
              your own role
            </Text>
            , which will cause you to lose privileges within the{" "}
            <Text span fw="bold">
              {currentOrganization.name}
            </Text>{" "}
            organization.
          </Text>
        </Alert>
      )}

      <Group justify="space-between">
        <Button variant="outline" onClick={cancel}>
          Cancel
        </Button>
        <Tooltip label={role_unchanged ? `${member.name} is already ${role_name}` : `Make ${member.name} ${role_name}`}>
          <Button onClick={() => submit(role)} disabled={role_unchanged}>
            Confirm
          </Button>
        </Tooltip>
      </Group>
    </Stack>
  );
}
