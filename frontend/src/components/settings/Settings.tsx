import { Button, Container, Grid, Group, PasswordInput, Stack, Text, TextInput, Tooltip } from "@mantine/core";
import { IconBrandGithub, IconPlugConnectedX, IconPlus, IconTrash, IconX } from "@tabler/icons-react";
import { useDisclosure } from "@mantine/hooks";
import { NewOrganization } from "../organizations/NewOrganization.tsx";
import GitHubBadge from "./GitHubBadge.tsx";
import { useForm } from "@mantine/form";
import { notifications } from "@mantine/notifications";
import { useRemails } from "../../hooks/useRemails.ts";
import SubscriptionCard from "./SubscriptionCard.tsx";

interface BasicFormValues {
  name: string;
  email: string;
}

interface PasswordForm {
  old_password: string;
  new_password1: string;
  new_password2: string;
}

export default function Settings() {
  const [opened, { open, close }] = useDisclosure(false);
  const { dispatch, navigate, state } = useRemails();
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

  const passwordForm = useForm<PasswordForm>({
    initialValues: {
      old_password: "",
      new_password1: "",
      new_password2: "",
    },
    validate: {
      old_password: (value) =>
        user.password_enabled && value.length <= 6 ? "Password should include at least 6 characters" : null,
      new_password1: (value) => (value.length <= 6 ? "Password should include at least 6 characters" : null),
      new_password2: (value, values) => (values.new_password1 !== value ? "Passwords do not match" : null),
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
        notifications.show({
          title: "Error",
          message: "Something went wrong",
          color: "red",
          autoClose: 20000,
          icon: <IconX size={20} />,
        });
      }
    });
  };

  const updatePassword = (update: PasswordForm) => {
    fetch(`/api/api_user/${user.id}/password`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ current_password: update.old_password, new_password: update.new_password1 }),
    }).then((res) => {
      if (res.status === 200) {
        passwordForm.reset();
        dispatch({ type: "set_user", user: { ...user, password_enabled: true } });
        notifications.show({
          title: "Updated",
          color: "green",
          message: "",
        });
      } else {
        notifications.show({
          title: "Error",
          message: "Something went wrong",
          color: "red",
          autoClose: 20000,
          icon: <IconX size={20} />,
        });
      }
    });
  };

  const removePassword = (update: PasswordForm) => {
    if (update.old_password.length <= 6) {
      passwordForm.setFieldError("old_password", "Password should include at least 6 characters");
      return;
    }
    if (update.new_password1 || update.new_password2) {
      passwordForm.setFieldError(
        "new_password1",
        "Watch out, you clicked to remove your password but provided a new one."
      );
      return;
    }

    fetch(`/api/api_user/${user.id}/password`, {
      method: "DELETE",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ current_password: update.old_password }),
    }).then((res) => {
      if (res.status === 200) {
        passwordForm.reset();
        dispatch({ type: "set_user", user: { ...user, password_enabled: false } });
        notifications.show({
          title: "Removed Password login",
          color: "green",
          message: "",
        });
      } else {
        notifications.show({
          title: "Error",
          message: "Something went wrong",
          color: "red",
          autoClose: 20000,
          icon: <IconX size={20} />,
        });
      }
    });
  };

  return (
    <Container size="xs" ml="0" pl="0">
      <Stack>
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
        <h2>Login mechanisms</h2>
        <Text>GitHub</Text>
        {user.github_id && (
          <Group>
            <GitHubBadge user_id={user.github_id} />
            <Tooltip
              label={
                user.password_enabled
                  ? "Remove Github as login method. You can still sign in with the email and password"
                  : "Please set up a password first to not loose access to your account"
              }
            >
              <Button
                onClick={disconnectGithub}
                leftSection={<IconPlugConnectedX />}
                variant="outline"
                disabled={!user.password_enabled}
              >
                Disconnect
              </Button>
            </Tooltip>
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

        <Text>Password</Text>
        <form onSubmit={passwordForm.onSubmit(updatePassword)}>
          <Stack>
            {user.password_enabled && (
              <PasswordInput
                label="Current password"
                key={passwordForm.key("old_password")}
                value={passwordForm.values.old_password}
                placeholder="Your current password"
                error={passwordForm.errors.old_password}
                onChange={(event) => passwordForm.setFieldValue("old_password", event.currentTarget.value)}
              />
            )}
            <PasswordInput
              label="New password"
              key={passwordForm.key("new_password1")}
              value={passwordForm.values.new_password1}
              placeholder="Your new password"
              error={passwordForm.errors.new_password1}
              onChange={(event) => passwordForm.setFieldValue("new_password1", event.currentTarget.value)}
            />
            <PasswordInput
              label="Repeat the new password"
              key={passwordForm.key("new_password2")}
              value={passwordForm.values.new_password2}
              placeholder="Repeat the new password"
              error={passwordForm.errors.new_password2}
              onChange={(event) => passwordForm.setFieldValue("new_password2", event.currentTarget.value)}
            />
            <Grid justify="space-between">
              {user.password_enabled && (
                <Grid.Col span={{ base: 12, sm: 6 }}>
                  <Tooltip
                    label={
                      user.github_id
                        ? "Delete your password. You will not be able to sign in with your email and password anymore"
                        : "You need to connect with Github first"
                    }
                  >
                    <Button
                      disabled={!user.github_id}
                      fullWidth={true}
                      onClick={() => removePassword(passwordForm.values)}
                      variant="outline"
                      leftSection={<IconTrash />}
                    >
                      Delete Password
                    </Button>
                  </Tooltip>
                </Grid.Col>
              )}
              <Grid.Col span={user.password_enabled ? { base: 12, sm: 6 } : 12}>
                <Button type="submit" fullWidth={true}>
                  {user.password_enabled ? "Update Password" : "Create Password"}
                </Button>
              </Grid.Col>
            </Grid>
          </Stack>
        </form>

        <h2>Organization Settings</h2>

        <SubscriptionCard />

        <NewOrganization
          opened={opened}
          close={close}
          done={(newOrg) => {
            navigate("settings", { org_id: newOrg.id });
          }}
        />
        <Button onClick={() => open()} leftSection={<IconPlus />}>
          New Organization
        </Button>
      </Stack>
    </Container>
  );
}
