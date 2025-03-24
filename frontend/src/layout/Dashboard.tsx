import { AppShell, Burger, Button, Flex, Group } from '@mantine/core';
import { useDisclosure } from '@mantine/hooks';
import logo from '../img/logo.png';
import ColorTheme from './ColorTheme';
import { IconLogout, IconUser } from '@tabler/icons-react';
import { useUser } from '../hooks/useUser';
import { Menu } from './Menu';
import { ReactNode } from 'react';

interface DashboardProps {
  children: ReactNode;
}

export function Dashboard({ children }: DashboardProps) {
  const [opened, { toggle }] = useDisclosure();
  const user = useUser();

  return (
    <AppShell
      header={{ height: 70 }}
      navbar={{ width: 250, breakpoint: 'sm', collapsed: { mobile: !opened } }}
      padding="lg"
    >
      <AppShell.Header>
        <Flex align="center" h="100%" justify="space-between">
          <Group h="100%" px="lg">
            <Burger opened={opened} onClick={toggle} hiddenFrom="sm" size="sm" />
            <img src={logo} alt="Logo" style={{ height: 40 }} />
          </Group>
          <Group h="100%" px="lg">
            <ColorTheme />
            <Button
              leftSection={<IconUser />}
              color="#666"
              variant="outline"
            >
              {user.email}
            </Button>
            <Button
              leftSection={<IconLogout />}
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
        <Menu />
      </AppShell.Navbar>
      <AppShell.Main>
        {children}
      </AppShell.Main>
    </AppShell>
  );
}
