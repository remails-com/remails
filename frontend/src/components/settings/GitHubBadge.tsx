import {Avatar, Group, Paper, Stack, Text} from "@mantine/core";
import {useEffect, useState} from "react";

interface GitHubBadgeProps {
  user_id: string;
}

interface GitHubApi {
  login: string;
  avatar_url: string;
  name: string;
  html_url: string;
}

export default function GitHubBadge({user_id}: GitHubBadgeProps) {
  const [user, setUser] = useState<GitHubApi | null>(null);

  useEffect(() => {
    fetch(`https://api.github.com/user/${user_id}`)
      .then(res => res.json())
      .then(data => setUser(data))
      .catch(err => console.error(err))
  }, []);


  return (
      <Paper component="a" href={user?.html_url} radius="xl" p="lg" style={{background: "var(--mantine-color-black)"}}>
        <Group>
          <Avatar src={user?.avatar_url} size="lg"/>
          <Stack gap="xs">
            <Text size="sm" c="white" fw={600}>
              {user?.name}
            </Text>
            <Text c="white" size="xs">
              {user?.login}
            </Text>
          </Stack>
        </Group>
      </Paper>
  )
}