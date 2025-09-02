import { AppShell, Box, Burger, Button, Flex, Group, Menu } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import ColorTheme from "./ColorTheme";
import { IconChevronDown, IconLogout, IconSettings, IconUser, IconUserBolt } from "@tabler/icons-react";
import { NavBar } from "./NavBar.tsx";
import { ReactNode } from "react";
import { useRemails } from "../hooks/useRemails.ts";
import { useOrganizations } from "../hooks/useOrganizations.ts";
import { RemailsLogo } from "../components/RemailsLogo.tsx";
import { VersionInfo } from "./VersionInfo.tsx";
import { Link } from "../Link.tsx";

interface DashboardProps {
  children: ReactNode;
}

export function Dashboard({ children }: DashboardProps) {
  const [navbarOpened, { toggle, close }] = useDisclosure();
  const {
    state: { user },
    navigate,
  } = useRemails();
  const { currentOrganization } = useOrganizations();

  if (!user) {
    return null;
  }

  const isAdmin =
    user.global_role == "admin" ||
    user.org_roles.some((role) => role.role == "admin" && role.org_id == currentOrganization?.id);

  const user_dropdown = (
    <Menu width={260} position="bottom-start" transitionProps={{ transition: "fade-down" }} withinPortal>
      <Menu.Target>
        <Button
          rightSection={<IconChevronDown size={20} stroke={1.8} />}
          color="#666"
          variant="outline"
          fullWidth
          justify="space-between"
          px="sm"
        >
          <Box mr="sm">{isAdmin ? <IconUserBolt /> : <IconUser />}</Box>
          {user.name}
        </Button>
      </Menu.Target>
      <Menu.Dropdown>
        <Menu.Item leftSection={<IconSettings size={20} stroke={1.8} />} onClick={() => navigate("account")}>
          User settings
        </Menu.Item>
        <Menu.Item leftSection={<IconLogout size={20} stroke={1.8} />} c="remails-red" href="/api/logout" component="a">
          Log out
        </Menu.Item>
      </Menu.Dropdown>
    </Menu>
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
            <Link to="projects">
              <RemailsLogo />
            </Link>
          </Group>
          <Group h="100%" px="lg">
            <ColorTheme />
            <Group h="100%" visibleFrom="sm">
              {user_dropdown}
            </Group>
          </Group>
        </Flex>
      </AppShell.Header>
      <AppShell.Navbar p="md">
        <Group hiddenFrom="sm" pb="md">
          {user_dropdown}
        </Group>

        <NavBar close={close} />
      </AppShell.Navbar>
      <AppShell.Main style={{ display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
        <Box style={{ position: "relative" }}>{children}</Box>

        <Group mt="xl" justify="right">
          <VersionInfo />
        </Group>
      </AppShell.Main>
    </AppShell>
  );
}
