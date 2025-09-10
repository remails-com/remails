import { ActionIcon, Box, Button, Center, Group, Stack, Text, Tooltip } from "@mantine/core";
import { useSubscription } from "../hooks/useSubscription.ts";
import { RemailsLogo } from "./RemailsLogo.tsx";
import { IconReload } from "@tabler/icons-react";
import { useOrganizations } from "../hooks/useOrganizations.ts";

export default function SetupSubscription() {
  const { subscription, reloadSubscription, navigateToSales } = useSubscription();
  const { currentOrganization } = useOrganizations();

  if (!currentOrganization || !subscription) {
    return null;
  }

  return (
    <Center>
      <Box maw="25rem">
        <Text size="xl" fw="bold">
          Welcome to
        </Text>
        <Group justify="center" mt="sm">
          <RemailsLogo />
        </Group>
        <Stack mt="lg">
          <Text>Before getting started. please register your company details. No worries, there is a free tier.</Text>
          <Group>
            <Button flex={1} onClick={navigateToSales}>
              Choose your subscription
            </Button>
            <Tooltip label={"Reload subscription data"}>
              <ActionIcon variant="outline" size="input-sm">
                <IconReload onClick={reloadSubscription} />
              </ActionIcon>
            </Tooltip>
          </Group>
        </Stack>
      </Box>
    </Center>
  );
}
