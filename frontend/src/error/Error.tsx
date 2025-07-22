import { Alert, Anchor, Stack, Text, Title } from "@mantine/core";
import { RemailsError } from "./error";
import { IconExclamationCircle } from "@tabler/icons-react";

export default function Error({ error }: { error: RemailsError }) {
  return (
    <Stack align="center" py="80">
      <Text c="dimmed" fz="50" fw="bold">
        {error.status}
      </Text>
      <Title fw="bold" fz="38">
        You have found a secret place.
      </Title>
      <Text c="dimmed" size="lg" ta="center" maw="520" my="xl">
        Unfortunately, this is only an error page. You may have mistyped the address, or the page has been moved to
        another URL.
      </Text>
      <Alert icon={<IconExclamationCircle />}>{error.message}</Alert>
      <Anchor href="/" my="xl">
        Take me back home
      </Anchor>
    </Stack>
  );
}
