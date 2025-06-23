import { AppShell, Box, Burger, Button, Flex, Group, Menu, Text } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import ColorTheme from "./ColorTheme";
import { IconChevronDown, IconLogout, IconUser, IconUserBolt } from "@tabler/icons-react";
import { useUser } from "../hooks/useUser";
import { NavBar } from "./NavBar.tsx";
import { ReactNode, useState } from "react";
import { useRemails } from "../hooks/useRemails.ts";
import { useOrganizations } from "../hooks/useOrganizations.ts";
import { Breadcrumbs } from "./Breadcrumbs.tsx";
import { RemailsLogo } from "../components/RemailsLogo.tsx";
import { VersionInfo } from "./VersionInfo.tsx";

interface DashboardProps {
  children: ReactNode;
}

export function Dashboard({ children }: DashboardProps) {
  const [navbarOpened, { toggle, close }] = useDisclosure();
  const [_, setUserMenuOpened] = useState(false);
  const { user } = useUser();
  const {
    state: { organizations },
    navigate,
  } = useRemails();
  const { currentOrganization } = useOrganizations();

  const isAdmin = user.roles.some(
    (role) => role.type == "super_admin" || (role.type == "organization_admin" && role.id == currentOrganization?.id)
  );

  const org_switching = (
    <>
      <Menu
        width={260}
        position="bottom-start"
        transitionProps={{ transition: "fade-down" }}
        onClose={() => setUserMenuOpened(false)}
        onOpen={() => setUserMenuOpened(true)}
        withinPortal
      >
        <Menu.Target>
          <Button
            leftSection={isAdmin ? <IconUserBolt /> : <IconUser />}
            rightSection={<IconChevronDown size={20} stroke={1.8} />}
            color="#666"
            variant="outline"
          >
            {user.name}
          </Button>
        </Menu.Target>
        <Menu.Dropdown>
          {organizations
            ?.filter((all_org) => {
              return (
                user.roles.find((role) => {
                  return role.type === "organization_admin" && role.id === all_org.id;
                }) || all_org.id === currentOrganization?.id
              );
            })
            .map((org) => (
              <Menu.Item key={org.id} value={org.id} onClick={() => navigate("projects", { org_id: org.id })}>
                <Text fw={org.id === currentOrganization?.id ? 700 : 400}>{org.name}</Text>
              </Menu.Item>
            ))}
        </Menu.Dropdown>
      </Menu>
      <Button leftSection={<IconLogout />} variant="light" component="a" href="/api/logout">
        Logout
      </Button>
    </>
  );

  return (
    <AppShell
      header={{ height: 70 }}
      navbar={{ width: 250, breakpoint: "sm", collapsed: { mobile: !navbarOpened } }}
      padding="lg"
    >
      <AppShell.Header>
        <Flex align="center" h="100%" justify="space-between">
          <Group h="100%" px="lg" wrap="nowrap">
            <Burger opened={navbarOpened} onClick={toggle} hiddenFrom="sm" size="sm" />
            <RemailsLogo />
          </Group>
          <Group h="100%" px="lg">
            <ColorTheme />
            <Group h="100%" visibleFrom="sm">
              {org_switching}
            </Group>
          </Group>
        </Flex>
      </AppShell.Header>
      <AppShell.Navbar p="md">
        <Group hiddenFrom="sm" pb="lg">
          {org_switching}
        </Group>
        <NavBar close={close} />
      </AppShell.Navbar>
      <AppShell.Main style={{ display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
        <Box>
          <Breadcrumbs />
          {children}
        </Box>

        <Group mt="xl" justify="right">
          <VersionInfo />
        </Group>
      </AppShell.Main>
    </AppShell>
  );
}
