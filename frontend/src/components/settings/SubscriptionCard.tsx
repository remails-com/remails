import { Badge, Button, Card, Divider, Grid, Skeleton, Stack, Text, Tooltip } from "@mantine/core";
import { useSubscription } from "../../hooks/useSubscription.ts";
import { Subscription, SubscriptionStatus } from "../../types.ts";
import React from "react";

export default function SubscriptionCard() {
  const { subscription } = useSubscription();

  const details = (subscription: SubscriptionStatus) => {
    if (subscription.status === "none") {
      return no_subscription;
    } else {
      return existing_subscription(subscription);
    }
  };

  const no_subscription = <Text size="xl">No subscription found</Text>;

  const existing_subscription = (subscription: Subscription) => (
    <>
      <Grid justify="space-between">
        <Grid.Col span="auto">
          <Text size="xl">{subscription.title}</Text>
        </Grid.Col>
        <Grid.Col span="content">
          <Badge size="lg" color="green" variant="light">
            Active{subscription.end_date ? `until ${subscription.end_date}` : ""}
          </Badge>
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
      <Tooltip label="You will need a code that was send to you via email">
        <Button component="a" href={subscription.sales_invoices_url} target="_blank">
          Manage invoices, subscription, and contact details
        </Button>
      </Tooltip>
    </>
  );

  const details_skeleton = (
    <>
      <Skeleton height="1.7rem" width="20%" />
      <Divider />
      <Skeleton height="1rem" width="80%" />
      <Skeleton height="1rem" width="80%" />
      <Skeleton height="1rem" width="80%" />
      <Button variant="light" />
    </>
  );

  return (
    <Card shadow="sm" padding="lg" radius="md" withBorder>
      <Stack gap="md">{subscription === null ? details_skeleton : details(subscription)}</Stack>
    </Card>
  );
}
