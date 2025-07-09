import { Button, Group, Stack, Text, Title } from "@mantine/core";
import classes from "./NotFound.module.css";
import { Link } from "../Link.tsx";

export function NotFound() {
  return (
    <Stack align="center" className={classes.root}>
      <div className={classes.label}>404</div>
      <Title className={classes.title}>You have found a secret place.</Title>
      <Text c="dimmed" size="lg" ta="center" className={classes.description}>
        Unfortunately, this is only a 404 page. You may have mistyped the address, or the page has been moved to another
        URL.
      </Text>
      <Group justify="center">
        <Button variant="subtle" size="md" mt="lg">
          <Link to="default" underline="never">Take me back home</Link>
        </Button>
      </Group>
    </Stack>
  );
}
