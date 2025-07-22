import { ActionIcon, Anchor, Avatar, Box, Group, Paper, Stack, Text, Tooltip } from "@mantine/core";
import { IconExternalLink, IconPlugConnectedX } from "@tabler/icons-react";
import { useEffect, useState } from "react";

interface GitHubBadgeProps {
  user_id: string;
  disconnect_github: () => void;
  can_disconnect: boolean;
}

interface GitHubApi {
  login: string;
  avatar_url: string;
  name: string;
  html_url: string;
}

export default function GitHubBadge({ user_id, disconnect_github, can_disconnect }: GitHubBadgeProps) {
  const [user, setUser] = useState<GitHubApi | null>(null);

  useEffect(() => {
    fetch(`https://api.github.com/user/${user_id}`)
      .then((res) => res.json())
      .then((data) => setUser(data))
      .catch((err) => console.error(err));
  }, [user_id]);

  return (
    <Paper radius="200" p="sm" style={{ background: "var(--mantine-color-black)" }} w="100%">
      <Group justify="space-between">
        <Group>
          <Avatar src={user?.avatar_url} size="lg" />
          <Stack gap="0">
            <Text size="md" c="white" fw={600}>
              {user?.name}
            </Text>
            <Anchor c="white" size="sm" href={user?.html_url} target="_blank">
              {user?.login}
              <Box ml="0.5ch" component="span">
                <IconExternalLink size="14" />
              </Box>
            </Anchor>
          </Stack>
        </Group>
        <Tooltip
          label={
            can_disconnect
              ? "Remove Github as login method. You can still sign in with the email and password"
              : "Please set up a password first to not loose access to your account"
          }
        >
          <ActionIcon onClick={disconnect_github} disabled={!can_disconnect} size="xl" mr="xs" bdrs="100%">
            <IconPlugConnectedX />
          </ActionIcon>
        </Tooltip>
      </Group>
    </Paper>
  );
}
