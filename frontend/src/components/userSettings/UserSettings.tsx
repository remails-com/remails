import { Button, Container, Group, Stack, Tabs, TextInput, Title } from "@mantine/core";
import GitHubBadge from "./GitHubBadge";
import { IconAuth2fa, IconBrandGithub, IconPassword } from "@tabler/icons-react";
import { notifications } from "@mantine/notifications";
import { useRemails } from "../../hooks/useRemails";
import { useForm } from "@mantine/form";
import { errorNotification } from "../../notify";
import TotpSetup from "./TotpSetup.tsx";
import { useDisclosure } from "@mantine/hooks";
import { PasswordSettings } from "./PasswordSettings.tsx";
import TotpList from "./TotpList.tsx";

interface BasicFormValues {
  name: string;
  email: string;
}

export default function UserSettings() {
  const { dispatch, state } = useRemails();
  const [mfaOpened, { open: openMfa, close: closeMfa }] = useDisclosure(false);
  const user = state.user!;

  const basicForm = useForm<BasicFormValues>({
    initialValues: {
      name: user.name,
      email: user.email,
    },
    validate: {
      name: (value) => (value.length < 3 ? "Name must have at least 3 letters" : null),
      email: (value) => (/^\S+@\S+$/.test(value) ? null : "Invalid email"),
    },
  });

  const disconnectGithub = () => {
    fetch("/api/login/github", {
      method: "DELETE",
    }).then((res) => {
      if (res.status === 200) {
        res.json().then((user) => {
          dispatch({ type: "set_user", user });
        });
      }
    });
  };

  const updateUser = (update: BasicFormValues) => {
    fetch(`/api/api_user/${user.id}`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(update),
    }).then((res) => {
      if (res.status === 200) {
        res.json().then((user) => {
          dispatch({ type: "set_user", user });
          basicForm.resetDirty();
          notifications.show({
            title: "Updated",
            color: "green",
            message: "",
          });
        });
      } else {
        errorNotification("User could not be updated");
        console.error(res);
      }
    });
  };

  return (
    <Container size="xs" ml="0" pl="0">
      <Stack>
        <Title order={2}>User Settings</Title>
        <form onSubmit={basicForm.onSubmit(updateUser)}>
          <Stack>
            <TextInput
              label="Name"
              key={basicForm.key("name")}
              value={basicForm.values.name}
              placeholder="Your name"
              error={basicForm.errors.name}
              onChange={(event) => basicForm.setFieldValue("name", event.currentTarget.value)}
            />
            <TextInput
              label="Email"
              type="email"
              key={basicForm.key("email")}
              value={basicForm.values.email}
              placeholder="your@email.com"
              error={basicForm.errors.email}
              onChange={(event) => basicForm.setFieldValue("email", event.currentTarget.value)}
            />
            <Button disabled={!basicForm.isDirty()} type="submit">
              Save
            </Button>
          </Stack>
        </form>

        <Title order={3}>Login mechanisms</Title>

        <Tabs defaultValue="password">
          <Tabs.List>
            <Tabs.Tab value="password" leftSection={<IconPassword size={14} />}>
              Password
            </Tabs.Tab>
            <Tabs.Tab value="2fa" leftSection={<IconAuth2fa size={14} />}>
              Two Factor Authentication
            </Tabs.Tab>
            <Tabs.Tab value="github" leftSection={<IconBrandGithub size={14} />}>
              GitHub
            </Tabs.Tab>
          </Tabs.List>

          <Tabs.Panel value="password" mt="md">
            <PasswordSettings />
          </Tabs.Panel>
          <Tabs.Panel value="2fa" mt="md">
            <TotpSetup opened={mfaOpened} close={closeMfa} />
            <Stack>
              <TotpList />
              <Button onClick={openMfa}>Add Authenticator App</Button>
            </Stack>
          </Tabs.Panel>

          <Tabs.Panel value="github" mt="md">
            {user.github_id && (
              <Group>
                <GitHubBadge
                  user_id={user.github_id}
                  can_disconnect={user.password_enabled}
                  disconnect_github={disconnectGithub}
                />
              </Group>
            )}
            {!user.github_id && (
              <Button
                size="xl"
                radius="xl"
                leftSection={<IconBrandGithub />}
                variant="filled"
                color="black"
                component="a"
                href="/api/login/github"
              >
                Connect with Github
              </Button>
            )}
          </Tabs.Panel>
        </Tabs>
      </Stack>
    </Container>
  );
}
