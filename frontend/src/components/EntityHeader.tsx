import { Divider, Group, Stack, Text, ThemeIcon, Title } from "@mantine/core";
import { Icon, IconProps } from "@tabler/icons-react";
import Rename from "./Rename";

interface EntityHeaderProps {
  name: string;
  entityType: string;
  saveRename?: (values: { name: string }) => Promise<void>;
  Icon: React.ForwardRefExoticComponent<IconProps & React.RefAttributes<Icon>>;
  divider?: boolean; // we don't want the divider if we are using tabs
}

export default function EntityHeader({ name, entityType, saveRename, Icon, divider = false }: EntityHeaderProps) {
  return (
    <>
      <Group bg="var(--mantine-color-gray-light)" p="lg" mx="-lg" mt="-lg">
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
      {divider && <Divider mx="-lg" mb="md" />}
    </>
  );
}
