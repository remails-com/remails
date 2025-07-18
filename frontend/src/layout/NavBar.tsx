import { Box, Button, Menu, NavLink, Text } from "@mantine/core";
import {
  IconAffiliate,
  IconBuildings,
  IconChartBar,
  IconChevronDown,
  IconPlus,
  IconServer,
  IconSettings,
  IconWorldWww,
} from "@tabler/icons-react";
import { is_global_admin } from "../util.ts";
import { useRemails } from "../hooks/useRemails.ts";
import { useOrganizations } from "../hooks/useOrganizations.ts";
import { useDisclosure } from "@mantine/hooks";
import { NewOrganization } from "../components/organizations/NewOrganization.tsx";

export function NavBar({ close }: { close: () => void }) {
  const {
    state: { routerState, user, organizations },
    navigate,
  } = useRemails();
  const { currentOrganization } = useOrganizations();
  const [openedNewOrg, { open: openNewOrg, close: closeNewOrg }] = useDisclosure(false);

  if (!user) {
    return null;
  }

  const roles = user.roles || [];

  return (
    <>
      {is_global_admin(roles) && (
        <NavLink
          label="Organizations"
          active={routerState.name === "organizations"}
          leftSection={<IconBuildings size={20} stroke={1.8} />}
          onClick={() => {
            navigate("organizations");
            close();
          }}
        />
      )}

      <NewOrganization
        opened={openedNewOrg}
        close={closeNewOrg}
        done={(newOrg) => {
          navigate("settings", { org_id: newOrg.id });
        }}
      />

      <Menu width={260} position="bottom-start" transitionProps={{ transition: "fade-down" }} withinPortal>
        <Menu.Target>
          <Button
            rightSection={<IconChevronDown size={20} stroke={1.8} />}
            color="#666"
            variant="outline"
            justify="space-between"
            fullWidth
            px="sm"
            mb="md"
          >
            <Box mr="sm">
              <IconAffiliate />
            </Box>
            {currentOrganization?.name}
          </Button>
        </Menu.Target>
        <Menu.Dropdown>
          <Menu.Label>Select organization</Menu.Label>
          {organizations
            ?.filter((all_org) => {
              return (
                user.roles.find((role) => {
                  return role.type === "organization_admin" && role.id === all_org.id;
                }) || all_org.id === currentOrganization?.id
              );
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

      <NavLink
        label="Projects"
        active={routerState.name.startsWith("projects")}
        leftSection={<IconServer size={20} stroke={1.8} />}
        onClick={() => {
          navigate("projects");
          close();
        }}
      />
      <NavLink
        label="Domains"
        active={routerState.name.startsWith("domains")}
        leftSection={<IconWorldWww size={20} stroke={1.8} />}
        onClick={() => {
          navigate("domains");
          close();
        }}
      />
      <NavLink
        label="Statistics"
        active={routerState.name === "statistics"}
        leftSection={<IconChartBar size={20} stroke={1.8} />}
        onClick={() => {
          navigate("statistics");
          close();
        }}
      />
      <NavLink
        label="Settings"
        active={routerState.name === "settings"}
        leftSection={<IconSettings size={20} stroke={1.8} />}
        onClick={() => {
          navigate("settings");
          close();
        }}
      />
    </>
  );
}
