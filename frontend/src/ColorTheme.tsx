import { ActionIcon, useMantineColorScheme } from "@mantine/core";
import { IconMoonStars, IconSun } from '@tabler/icons-react';

export default function ColorTheme() {
  const { colorScheme, toggleColorScheme } = useMantineColorScheme({
    keepTransitions: true,
  });

  return (
    <ActionIcon
      variant='transparent'
      onClick={() => toggleColorScheme()}
      size='xl'
    >
      {colorScheme === 'dark' ? (
        <IconSun size={16} />
      ) : (
        <IconMoonStars size={16}  />
      )}
    </ActionIcon>
  );
}
