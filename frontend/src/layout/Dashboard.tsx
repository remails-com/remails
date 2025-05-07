import {AppShell, Burger, Button, Flex, Group, Menu, Text} from '@mantine/core';
import {useDisclosure} from '@mantine/hooks';
import logo from '../img/logo.png';
import ColorTheme from './ColorTheme';
import {IconChevronDown, IconLogout, IconUser} from '@tabler/icons-react';
import {useUser} from '../hooks/useUser';
import {NavBar} from './NavBar.tsx';
import {ReactNode, useState} from 'react';
import {useRemails} from "../hooks/useRemails.ts";
import {useOrganizations} from '../hooks/useOrganizations.ts';
import {Breadcrumbs} from "./Breadcrumbs.tsx";

interface DashboardProps {
  children: ReactNode;
}

export function Dashboard({children}: DashboardProps) {
  const [navbarOpened, {toggle}] = useDisclosure();
  const [_, setUserMenuOpened] = useState(false);
  const {user} = useUser();
  const {state: {organizations}, navigate} = useRemails();
  const {currentOrganization} = useOrganizations();

  return (
    <AppShell
      header={{height: 70}}
      navbar={{width: 250, breakpoint: 'sm', collapsed: {mobile: !navbarOpened}}}
      padding="lg"
    >
      <AppShell.Header>
        <Flex align="center" h="100%" justify="space-between">
          <Group h="100%" px="lg">
            <Burger opened={navbarOpened} onClick={toggle} hiddenFrom="sm" size="sm"/>
            <img src={logo} alt="Logo" style={{height: 40}}/>
          </Group>
          <Group h="100%" px="lg">
            <ColorTheme/>
            <Menu
              width={260}
              position="bottom-start"
              transitionProps={{transition: 'fade-down'}}
              onClose={() => setUserMenuOpened(false)}
              onOpen={() => setUserMenuOpened(true)}
              withinPortal
            >
              <Menu.Target>
                <Button
                  leftSection={<IconUser/>}
                  rightSection={<IconChevronDown size={20} stroke={1.8}/>}
                  color="#666"
                  variant="outline"
                >
                  {user.name}
                </Button>
              </Menu.Target>
              <Menu.Dropdown>
                {organizations?.map((org) => (
                  <Menu.Item key={org.id} value={org.id} onClick={() => navigate('projects', {org_id: org.id})}>
                    <Text fw={org.id === currentOrganization?.id ? 700 : 400}>{org.name}</Text>
                  </Menu.Item>
                ))}

              </Menu.Dropdown>

            </Menu>
            <Button
              leftSection={<IconLogout/>}
              variant="light"
              component="a"
              href="/api/logout"
            >
              Logout
            </Button>
          </Group>
        </Flex>
      </AppShell.Header>
      <AppShell.Navbar p="md">
        <NavBar/>
      </AppShell.Navbar>
      <AppShell.Main>
        <Breadcrumbs/>
        {children}
      </AppShell.Main>
    </AppShell>
  );
}
