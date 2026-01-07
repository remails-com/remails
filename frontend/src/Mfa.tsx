import { User } from "./types.ts";
import { Alert, Button, Center, FocusTrap, Group, Paper, PinInput, Stack, Text } from "@mantine/core";
import { RemailsLogo } from "./components/RemailsLogo.tsx";
import { useRemails } from "./hooks/useRemails.ts";
import { useForm } from "@mantine/form";
import { useState } from "react";

interface MfaProps {
  setUser: (user: User | null) => void;
}

interface FormValues {
  code: string;
}

export default function Mfa({ setUser }: MfaProps) {
  const { redirect } = useRemails();

  const [error, setError] = useState<string | null>(null);

  const form = useForm<FormValues>({
    initialValues: {
      code: "",
    },
    validate: {
      code: (val) => val.length !== 6,
    },
  });

  const submit = async ({ code }: FormValues) => {
    const res = await fetch("/api/login/totp", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(code),
    });
    if (res.status === 401) {
      setError("Invalid code");
      form.setFieldError("code", true);
      return;
    }
    if (res.status === 429) {
      setError("Too many attempts, please try again later");
      form.setFieldError("code", true);
      return;
    }
    if (res.status === 200) {
      const user: User = await res.json();
      setUser(user);
      redirect();
      return;
    }
    setError("Something went wrong, please try again later");
  };

  return (
    <Center mih="100vh">
      <Paper radius="md" p="xl" withBorder>
        <RemailsLogo height={45} />
        <form onSubmit={form.onSubmit(submit)}>
          <Stack mb="md" mt="md">
            <Text size="lg" fw="bold">
              Two-factor authentication
            </Text>
            <Text size="md" maw={300}>
              Enter the code from your two-factor authentication app or browser extension below.
            </Text>

            <FocusTrap>
              <PinInput
                value={form.values.code}
                key={form.key("code")}
                onChange={(value) => form.setFieldValue("code", value)}
                oneTimeCode
                error={!!form.errors.code}
                size="md"
                length={6}
                type="number"
              />
            </FocusTrap>
            {error && <Alert variant="light" title={error} />}
            <Group justify="space-between">
              <Button href="/api/logout" component="a" variant="outline">
                Cancel
              </Button>
              <Button type="submit" loading={form.submitting} disabled={!form.isValid()}>
                Sign-in
              </Button>
            </Group>
          </Stack>
        </form>
      </Paper>
    </Center>
  );
}
