import {
  Alert,
  Anchor,
  Button,
  Center,
  Divider,
  Group,
  Paper,
  PasswordInput,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { useForm } from "@mantine/form";
import { IconBrandGithub, IconX } from "@tabler/icons-react";
import { useState } from "react";
import { SignUpRequest, User, WhoamiResponse } from "./types.ts";
import { RemailsLogo } from "./components/RemailsLogo.tsx";
import { useRemails } from "./hooks/useRemails.ts";
import { notifications } from "@mantine/notifications";

interface LoginProps {
  setUser: (user: User | null) => void;
}

type LoginType = "login" | "register" | "reset_password";

const BUTTON_NAMES: { [type in LoginType]: string } = {
  login: "Login",
  register: "Register",
  reset_password: "Reset password",
};

export default function Login({ setUser }: LoginProps) {
  const {
    state: { routerState },
    navigate,
    redirect,
  } = useRemails();

  const type = routerState.params.type in BUTTON_NAMES ? (routerState.params.type as LoginType) : "login";
  const buttonName = BUTTON_NAMES[type];

  const [globalError, setGlobalError] = useState<string | null>(null);

  const form = useForm({
    initialValues: {
      email: "",
      name: "",
      password: "",
    },
    validate: {
      name: (val) => (type === "register" && val.trim().length === 0 ? "Name cannot be empty" : null),
      email: (val) => (/^\S+@\S+$/.test(val) ? null : "Invalid email"),
      password: (val) =>
        type !== "reset_password" && (val.length <= 10 ? "Password should include at least 10 characters" : null),
    },
  });

  const submit = async ({ email, name, password }: SignUpRequest) => {
    setGlobalError(null);
    if (type === "login") {
      const res = await fetch("/api/login/password", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ email, password }),
      });
      if (res.status === 404) {
        form.setFieldError("password", "Wrong username or password");
      } else if (res.status !== 200) {
        setGlobalError("Something went wrong");
      } else {
        const whoami: WhoamiResponse = await res.json();
        if (!whoami || "error" in whoami) {
          setGlobalError("Something went wrong");
          return;
        }
        if (whoami.login_status === "mfa_pending") {
          navigate("mfa", routerState.params);
        }
        redirect();
      }
    }

    if (type === "register") {
      const res = await fetch("/api/register/password", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ email, name, password }),
      });
      if (res.status === 409) {
        form.setFieldError("email", "This email is already registered, try logging in instead");
      } else if (res.status !== 201) {
        setGlobalError("Something went wrong");
      } else {
        const user = await res.json();
        setUser(user);
        redirect();
      }
    }

    if (type === "reset_password") {
      const res = await fetch("/api/login/password/reset", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(email),
      });
      if (res.status !== 200) {
        setGlobalError("Something went wrong");
      } else {
        notifications.show({
          title: "Please check your inbox",
          message: "If this email is registered at Remails you should receive a password reset link shortly",
          color: "green",
          autoClose: 20000,
        });
      }
    }
  };

  const github_login = routerState.params.redirect
    ? `/api/login/github?redirect=${encodeURIComponent(routerState.params.redirect)}`
    : "/api/login/github";

  return (
    <Center mih="100vh">
      <Paper radius="md" p="xl" withBorder w="450">
        <RemailsLogo height={45} />
        <Text size="md" mt="sm">
          Welcome! {type == "login" ? "Login" : "Register"} with:
        </Text>

        <Group grow mb="md" mt="md">
          <Button
            size="xl"
            radius="xl"
            leftSection={<IconBrandGithub />}
            variant="filled"
            color="black"
            component="a"
            href={github_login}
          >
            Github
          </Button>
        </Group>

        <Divider label={<Text size="sm">or continue with email:</Text>} labelPosition="center" my="lg" />

        <form onSubmit={form.onSubmit(submit)}>
          <Stack>
            {type === "register" && (
              <TextInput
                required
                label="Name"
                placeholder="Your name"
                value={form.values.name}
                onChange={(event) => form.setFieldValue("name", event.currentTarget.value)}
                error={form.errors.name}
                radius="md"
              />
            )}

            <TextInput
              required
              label="Email"
              placeholder="hello@remails.dev"
              value={form.values.email}
              onChange={(event) => form.setFieldValue("email", event.currentTarget.value)}
              error={form.errors.email}
              radius="md"
            />

            {type !== "reset_password" && (
              <PasswordInput
                required
                label="Password"
                placeholder="Your password"
                value={form.values.password}
                onChange={(event) => form.setFieldValue("password", event.currentTarget.value)}
                error={form.errors.password}
                radius="md"
              />
            )}

            {globalError && <Alert icon={<IconX size={20} />}>{globalError}</Alert>}
          </Stack>

          <Group justify="space-between" mt="xl">
            <Anchor
              component="button"
              type="button"
              onClick={() =>
                navigate(routerState.name, { ...routerState.params, type: type === "register" ? "login" : "register" })
              }
              size="sm"
            >
              {type === "register" ? "Already have an account? Login" : "Don't have an account? Register"}
            </Anchor>
            <Button type="submit" radius="xl" loading={form.submitting}>
              {buttonName}
            </Button>
          </Group>

          {type === "login" && (
            <Anchor
              c="dimmed"
              component="button"
              type="button"
              size="xs"
              onClick={() =>
                navigate(routerState.name, {
                  ...routerState.params,
                  type: "reset_password",
                })
              }
            >
              Forgot your password?
            </Anchor>
          )}

          {type === "register" && (
            <Text c="dimmed" size="sm" mt="xl">
              By signing up and using Remails you agree to our{" "}
              <Anchor href="https://www.remails.com/legal/terms.html" target="_blank">
                Terms and Conditions
              </Anchor>{" "}
              and our{" "}
              <Anchor href="https://www.remails.com/legal/privacy-statement.html" target="_blank">
                Privacy Policy
              </Anchor>
              .
            </Text>
          )}
        </form>
      </Paper>
    </Center>
  );
}
