import { Badge, Button, Card, Container, Divider, Grid, Stack, Text, Title, Tooltip } from "@mantine/core";
import { useSubscription } from "../../hooks/useSubscription.ts";
import { SubscriptionStatus } from "../../types.ts";
import React from "react";
import { formatDate } from "../../util.ts";
import { useOrgRole } from "../../hooks/useOrganizations.ts";

export default function SubscriptionCard() {
  const { subscription, navigateToSales, navigateToCustomerPortal } = useSubscription();
  const { isAdmin } = useOrgRole();

  if (!subscription) {
    return null;
  }

  const details = (subscription: SubscriptionStatus) => {
    if (subscription.status === "none") {
      return no_subscription;
    } else {
      return existing_subscription(subscription);
    }
  };

  const no_subscription = (
    <>
      <Text size="xl">No subscription found</Text>
      <Button onClick={navigateToSales}>Choose subscription</Button>
    </>
  );

  const existing_subscription = (subscription: Exclude<SubscriptionStatus, { status: "none" }>) => (
    <>
      <Grid justify="space-between">
        <Grid.Col span="auto">
          <Text size="xl">{subscription.title}</Text>
        </Grid.Col>
        <Grid.Col span="content">
          {subscription.status === "active" && (
            <Badge size="lg" color="green" variant="light" tt="none">
              Active{subscription.end_date ? ` until ${formatDate(subscription.end_date)}` : ""}
            </Badge>
          )}
          {subscription.status === "expired" && (
            <Badge size="lg" color="red" variant="light">
              Expired since {formatDate(subscription.end_date)}
            </Badge>
          )}
        </Grid.Col>
      </Grid>
      <Divider />
      <Text>
        {subscription.description.split("\n").map((line, i) => (
          <React.Fragment key={i}>
            {line}
            <br />
          </React.Fragment>
        ))}
      </Text>
      <Tooltip disabled={isAdmin} label="You need to be organization admin to manage the subscription.">
        <Button disabled={!isAdmin} onClick={navigateToCustomerPortal}>
          Manage invoices, subscription, and contact details
        </Button>
      </Tooltip>
      {subscription.status === "expired" && (
        <Tooltip disabled={isAdmin} label="You need to be organization admin to manage the subscription.">
          <Button disabled={!isAdmin} onClick={navigateToSales}>
            Choose new subscription
          </Button>
        </Tooltip>
      )}
    </>
  );

  return (
    <Container size="xs" mt="md" pl="0" ml="0">
      <Title order={3} mb="md">
        Your subscription
      </Title>
      <Card shadow="sm" padding="lg" radius="md" withBorder>
        <Stack gap="md">{details(subscription)}</Stack>
      </Card>
    </Container>
  );
}
