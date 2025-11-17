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
import { upperFirst } from "@mantine/hooks";
import { IconBrandGithub, IconX } from "@tabler/icons-react";
import { useState } from "react";
import { SignUpRequest, User, WhoamiResponse } from "./types.ts";
import { RemailsLogo } from "./components/RemailsLogo.tsx";
import { useRemails } from "./hooks/useRemails.ts";

interface LoginProps {
  setUser: (user: User | null) => void;
}

export default function Login({ setUser }: LoginProps) {
  const {
    state: { routerState },
    navigate,
    redirect,
  } = useRemails();

  const type: "login" | "register" = routerState.params.type === "register" ? "register" : "login";

  const [globalError, setGlobalError] = useState<string | null>(null);

  const xIcon = <IconX size={20} />;

  const form = useForm({
    initialValues: {
      email: "",
      name: "",
      password: "",
    },
    validate: {
      name: (val) => (type === "register" && val.trim().length === 0 ? "Name cannot be empty" : null),
      email: (val) => (/^\S+@\S+$/.test(val) ? null : "Invalid email"),
      password: (val) => (val.length <= 6 ? "Password should include at least 6 characters" : null),
    },
  });

  const submit = ({ email, name, password }: SignUpRequest) => {
    setGlobalError(null);
    if (type === "login") {
      fetch("/api/login/password", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ email, password }),
      }).then(async (res) => {
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
      });
    }

    if (type === "register") {
      fetch("/api/register/password", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ email, name, password }),
      }).then((res) => {
        if (res.status === 409) {
          form.setFieldError("email", "This email is already registered, try logging in instead");
        } else if (res.status !== 201) {
          setGlobalError("Something went wrong");
        } else {
          res.json().then(setUser);
          redirect();
        }
      });
    }
  };

  const github_login = routerState.params.redirect
    ? `/api/login/github?redirect=${encodeURIComponent(routerState.params.redirect)}`
    : "/api/login/github";

  return (
    <Center mih="100vh">
      <Paper radius="md" p="xl" withBorder w="400">
        <RemailsLogo style={{ height: 45 }} />
        <Text size="md">Welcome! {type == "login" ? "Login" : "Register"} with:</Text>

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

            <PasswordInput
              required
              label="Password"
              placeholder="Your password"
              value={form.values.password}
              onChange={(event) => form.setFieldValue("password", event.currentTarget.value)}
              error={form.errors.password}
              radius="md"
            />

            {globalError && <Alert icon={xIcon}>{globalError}</Alert>}
          </Stack>

          <Group justify="space-between" mt="xl">
            <Anchor
              component="button"
              type="button"
              onClick={() =>
                navigate(routerState.name, { ...routerState.params, type: type === "register" ? "login" : "register" })
              }
              // c="dimmed"
              // fw="bold"
              size="sm"
            >
              {type === "register" ? "Already have an account? Login" : "Don't have an account? Register"}
            </Anchor>
            <Button type="submit" radius="xl">
              {upperFirst(type)}
            </Button>
          </Group>

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
