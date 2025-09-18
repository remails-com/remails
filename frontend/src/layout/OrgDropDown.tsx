import { Box, Button, Menu, Text } from "@mantine/core";
import { IconBuildings, IconChevronDown, IconPlus } from "@tabler/icons-react";
import { useOrganizations } from "../hooks/useOrganizations.ts";
import { useRemails } from "../hooks/useRemails.ts";

export default function OrgDropDown({ openNewOrg }: { openNewOrg: () => void }) {
  const { currentOrganization, organizations } = useOrganizations();
  const {
    navigate,
    state: { user },
  } = useRemails();

  if (!user) {
    return null;
  }

  return (
    <Menu width={260} position="bottom-start" transitionProps={{ transition: "fade-down" }} withinPortal>
      <Menu.Target>
        <Button
          rightSection={<IconChevronDown size={20} stroke={1.8} />}
          color="#666"
          variant="outline"
          justify="space-between"
          fullWidth
          px="sm"
        >
          <Box mr="sm">
            <IconBuildings />
          </Box>
          {currentOrganization?.name}
        </Button>
      </Menu.Target>
      <Menu.Dropdown>
        <Menu.Label>Select organization</Menu.Label>
        {organizations
          ?.filter((all_org) => {
            return user.org_roles.find((role) => role.org_id === all_org.id) || all_org.id === currentOrganization?.id;
          })
          .map((org) => (
            <Menu.Item
              bg={org.id === currentOrganization?.id ? "var(--mantine-primary-color-light)" : ""}
              color={org.id === currentOrganization?.id ? "var(--mantine-primary-color-light-color)" : ""}
              key={org.id}
              value={org.id}
              onClick={() => navigate("projects", { org_id: org.id })}
            >
              <Text fs="italic">{org.name}</Text>
            </Menu.Item>
          ))}
        <Menu.Item onClick={() => openNewOrg()} leftSection={<IconPlus />}>
          New organization
        </Menu.Item>
      </Menu.Dropdown>
    </Menu>
  );
}
