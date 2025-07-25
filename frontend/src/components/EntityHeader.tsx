import { Group, Stack, Text, ThemeIcon, Title } from "@mantine/core";
import { Icon, IconProps } from "@tabler/icons-react";
import Rename from "./Rename";

interface EntityHeaderProps {
  name: string;
  entityType: string;
  saveRename?: (values: { name: string }) => Promise<void>;
  Icon: React.ForwardRefExoticComponent<IconProps & React.RefAttributes<Icon>>;
}

export default function EntityHeader({ name, entityType, Icon, saveRename }: EntityHeaderProps) {
  return (
    <Group mb="md">
      <ThemeIcon variant="light" size="xl">
        <Icon size="32" stroke="1.5" />
      </ThemeIcon>
      <Stack gap="0">
        <Text mb="0" c="remails-red" size="xs" tt="uppercase" fw="bold">
          {entityType}
        </Text>
        {saveRename ? (
          <Rename name={name} save={saveRename} />
        ) : (
          <Title order={3} mt="0">
            {name}
          </Title>
        )}
      </Stack>
    </Group>
  );
}
