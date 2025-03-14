import { AppShell, Burger, Button, Flex, Group, Skeleton } from '@mantine/core';
import { useDisclosure } from '@mantine/hooks';
import logo from './img/logo.png';
import ColorTheme from './ColorTheme';
import { IconLogout } from '@tabler/icons-react';

export default function Dashboard() {
  const [opened, { toggle }] = useDisclosure();

  return (
    <AppShell
      header={{ height: 60 }}
      navbar={{ width: 300, breakpoint: 'sm', collapsed: { mobile: !opened } }}
      padding="md"
    >
      <AppShell.Header>
        <Flex align="center" h="100%" justify="space-between">
          <Group h="100%" px="md">
            <Burger opened={opened} onClick={toggle} hiddenFrom="sm" size="sm" />
            <img src={logo} alt="Logo" style={{ height: 30 }} />
          </Group>
          <Group h="100%" px="md">
            <ColorTheme />
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
        Navbar
        {Array(15)
          .fill(0)
          .map((_, index) => (
            <Skeleton key={index} h={28} mt="sm" animate={false} />
          ))}
      </AppShell.Navbar>
      <AppShell.Main>Main</AppShell.Main>
    </AppShell>
  );
}
