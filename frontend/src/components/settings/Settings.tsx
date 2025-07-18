import { Container, Stack, Title } from "@mantine/core";
import SubscriptionCard from "./SubscriptionCard.tsx";

export default function Settings() {
  return (
    <Container size="xs" ml="0" pl="0">
      <Stack>
        <Title order={2}>Organization Settings</Title>
        <SubscriptionCard />
      </Stack>
    </Container>
  );
}
