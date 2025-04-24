import {
  Alert,
  Anchor,
  Button,
  Center,
  Checkbox,
  Divider,
  Group,
  Paper,
  PasswordInput,
  Stack,
  Text,
  TextInput,
} from '@mantine/core';
import {useForm} from '@mantine/form';
import {upperFirst, useToggle} from '@mantine/hooks';
import {IconBrandGithub, IconX} from '@tabler/icons-react';
import {useState} from "react";
import {SignUpRequest, WhoamiResponse} from "./types.ts";

interface LoginProps {
  setUser: (user: WhoamiResponse) => void;
}

export function Login({setUser}: LoginProps) {
  const [type, toggle] = useToggle(['login', 'register']);
  const [globalError, setGlobalError] = useState<string | null>(null);
  const xIcon = <IconX size={20}/>;

  const form = useForm({
    mode: 'controlled',
    initialValues: {
      email: '',
      name: '',
      password: '',
      terms: false,
    },
    onSubmitPreventDefault: 'always',
    validate: {
      name: (val) => ((type === 'register' && val.trim().length === 0) ? 'Name cannot be empty' : null),
      email: (val) => (/^\S+@\S+$/.test(val) ? null : 'Invalid email'),
      password: (val) => (val.length <= 6 ? 'Password should include at least 6 characters' : null),
      terms: (val) => (type === 'register' && !val),
    },
  });

  const submit = ({email, name, password, terms}: SignUpRequest) => {
    setGlobalError(null);
    if (type === 'login') {
      fetch('/api/login/password', {
        method: 'POST',
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({email, password})
      }).then(res => {
        if (res.status === 404) {
          form.setFieldError('password', 'Wrong username or password');
        } else if (res.status !== 200) {
          setGlobalError('Something went wrong');
        } else {
          res.json().then(setUser)
        }
      })
    }

    if (type === 'register') {
      fetch('/api/register/password', {
        method: 'POST',
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({email, name, password, terms,})
      }).then(res => {
        if (res.status === 409) {
          form.setFieldError('email', 'This email is already registered, try logging in instead');
        } else if (res.status !== 201) {
          setGlobalError('Something went wrong');
        } else {
          res.json().then(setUser)
        }
      })
    }
  }

  return (
    <Center mih="100vh">
      <Paper radius="md" p="xl" withBorder>
        <Text size="lg" fw={500}>
          Welcome to Rem@ils, {type} with
        </Text>

        <Group grow mb="md" mt="md">
          <Button
            size="xl"
            radius="xl"
            leftSection={<IconBrandGithub/>}
            variant="filled"
            color="black"
            component="a"
            href="/api/login/github"
          >
            Github
          </Button>
        </Group>

        <Divider label="Or continue with email" labelPosition="center" my="lg"/>

        <form onSubmit={form.onSubmit(submit)}>
          <Stack>
            {type === 'register' && (
              <TextInput
                label="Name"
                placeholder="Your name"
                value={form.values.name}
                onChange={(event) => form.setFieldValue('name', event.currentTarget.value)}
                error={form.errors.name}
                radius="md"
              />
            )}

            <TextInput
              required
              label="Email"
              placeholder="hello@remails.dev"
              value={form.values.email}
              onChange={(event) => form.setFieldValue('email', event.currentTarget.value)}
              error={form.errors.email}
              radius="md"
            />

            <PasswordInput
              required
              label="Password"
              placeholder="Your password"
              value={form.values.password}
              onChange={(event) => form.setFieldValue('password', event.currentTarget.value)}
              error={form.errors.password}
              radius="md"
            />

            {type === 'register' && (
              <Checkbox
                label="I accept terms and conditions"
                checked={form.values.terms}
                onChange={(event) => form.setFieldValue('terms', event.currentTarget.checked)}
                error={form.errors.terms}
              />
            )}

            {globalError &&
                <Alert icon={xIcon} color="red">
                  {globalError}
                </Alert>
            }
          </Stack>

          <Group justify="space-between" mt="xl">
            <Anchor component="button" type="button" c="dimmed" onClick={() => toggle()} size="xs">
              {type === 'register'
                ? 'Already have an account? Login'
                : "Don't have an account? Register"}
            </Anchor>
            <Button type="submit" radius="xl">
              {upperFirst(type)}
            </Button>
          </Group>
        </form>
      </Paper>
    </Center>
  );
}
