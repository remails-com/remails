import { Divider, Group, Stack, Text, ThemeIcon, Title, Flex, Box } from "@mantine/core";
import { Icon, IconProps } from "@tabler/icons-react";
import Rename from "./Rename";
import { Breadcrumbs } from "../layout/Breadcrumbs";
import classes from "./Header.module.css";
import { ReactNode } from "react";

interface HeaderProps {
  name: string;
  entityType: string;
  saveRename?: (values: { name: string }) => Promise<void>;
  Icon: React.ForwardRefExoticComponent<IconProps & React.RefAttributes<Icon>>;
  divider?: boolean; // we don't want the divider if we are using tabs
  addendum?: ReactNode;
}

export default function Header({ name, entityType, saveRename, Icon, divider = false, addendum }: HeaderProps) {
  return (
    <>
      <Group className={classes.header} px="lg" py="md" pt="xs" mx="-lg" mt="-lg">
        <Stack gap="md" w="100%">
          <Box w="100%">
            <Breadcrumbs />
            <Divider mt="xs" mx="-lg" />
          </Box>
          <Flex gap="sm">
            <ThemeIcon variant="light" size="xl">
              <Icon size="32" stroke="1.5" />
            </ThemeIcon>
            <Stack gap="0">
              <Text mb="0" c="remails-red" size="xs" tt="uppercase" fw="bold">
                {entityType}
              </Text>
              <Group>
                {saveRename ? (
                  <Rename name={name} save={saveRename} />
                ) : (
                  <Title order={3} mt="0">
                    {name}
                  </Title>
                )}
                {addendum}
              </Group>
            </Stack>
          </Flex>
        </Stack>
      </Group>
      {divider && <Divider mx="-lg" mb="md" />}
    </>
  );
}
