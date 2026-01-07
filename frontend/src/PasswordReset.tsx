import { useRemails } from "./hooks/useRemails.ts";
import { Anchor, Button, Center, Paper, PasswordInput, Stack, Text, TextInput } from "@mantine/core";
import { RemailsLogo } from "./components/RemailsLogo.tsx";
import { useForm } from "@mantine/form";
import { PasswordResetState } from "./types.ts";
import { useEffect, useState } from "react";
import { notifications } from "@mantine/notifications";
import { IconX } from "@tabler/icons-react";

interface FormValues {
  new_password: string;
  reset_secret: string;
  totp_code: string;
}

export default function PasswordReset() {
  const {
    state: { routerState },
    navigate,
  } = useRemails();
  const [state, setState] = useState<PasswordResetState | null>(null);

  const form = useForm<FormValues>({
    initialValues: {
      new_password: "",
      reset_secret: window.location.hash.substring(1),
      totp_code: "",
    },
    validate: {
      totp_code: (value) => {
        return state !== "ActiveWith2Fa" || value.match(/^\d{6}$/) ? null : "2FA code must be exactly 6 digits";
      },
      new_password: (value) => (value.length >= 10 ? null : "Password must be at least 10 characters long"),
    },
  });

  const submit = async (v: FormValues) => {
    const req = state === "ActiveWith2Fa" ? { ...v } : { ...v, totp_code: null };
    const res = await fetch(`/api/login/password/reset/${routerState.params.pw_reset_id}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(req),
    });

    if (res.status !== 200) {
      notifications.show({
        title: "Error",
        message: `Something went wrong. Is the link still valid?${state === "ActiveWith2Fa" ? " Is your 2FA code correct?" : ""}`,
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
      return;
    }
    notifications.show({
      title: "Password updated",
      message: "You can now log in with your new password",
      color: "green",
    });
    navigate("login");
  };

  useEffect(() => {
    fetch(`/api/login/password/reset/${routerState.params.pw_reset_id}`).then((res) => res.json().then(setState));
  }, [routerState.params.pw_reset_id]);

  if (!state) {
    return null;
  }

  return (
    <Center mih="100vh">
      <Paper radius="md" p="xl" withBorder w="450">
        <RemailsLogo height={45} />
        <Text size="md" mt="sm">
          Welcome!
        </Text>
        {state === "NotActive" ? (
          <>
            <Text mt="xl">
              This link does not seem to be valid (anymore). You can request a new one{" "}
              <Anchor component="a" type="button" inline onClick={() => navigate("login", { type: "reset_password" })}>
                here.
              </Anchor>
            </Text>
          </>
        ) : (
          <form onSubmit={form.onSubmit(submit)}>
            <Stack mt="xl">
              <PasswordInput
                required
                label="Password"
                placeholder="Your new password"
                value={form.values.new_password}
                onChange={(event) => form.setFieldValue("new_password", event.currentTarget.value)}
                error={form.errors.new_password}
                radius="md"
              />
              {state === "ActiveWith2Fa" && (
                <TextInput
                  required
                  value={form.values.totp_code}
                  label="Your 6 digit 2FA code"
                  key={form.key("totp_code")}
                  onChange={(event) => form.setFieldValue("totp_code", event.currentTarget.value)}
                  autoComplete="one-time-code"
                  radius="md"
                  error={form.errors.totp_code}
                  placeholder="e.g. 123456"
                  type="number"
                />
              )}
              <Button type="submit" mt="md">
                Reset password
              </Button>
            </Stack>
          </form>
        )}
      </Paper>
    </Center>
  );
}
