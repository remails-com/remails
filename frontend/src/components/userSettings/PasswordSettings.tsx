import { notifications } from "@mantine/notifications";
import { IconTrash } from "@tabler/icons-react";
import { useForm } from "@mantine/form";
import { useRemails } from "../../hooks/useRemails.ts";
import { Button, Grid, PasswordInput, Stack, Tooltip } from "@mantine/core";
import { errorNotification } from "../../notify.tsx";

interface PasswordForm {
  old_password: string;
  new_password1: string;
  new_password2: string;
}

export function PasswordSettings() {
  const { dispatch, state } = useRemails();

  const user = state.user!;

  const passwordForm = useForm<PasswordForm>({
    initialValues: {
      old_password: "",
      new_password1: "",
      new_password2: "",
    },
    validate: {
      old_password: (value) =>
        user.password_enabled && value.length <= 6 ? "Password should include at least 6 characters" : null,
      new_password1: (value) => (value.length <= 10 ? "Password should include at least 10 characters" : null),
      new_password2: (value, values) => (values.new_password1 !== value ? "Passwords do not match" : null),
    },
  });

  const updatePassword = async (update: PasswordForm) => {
    const res = await fetch(`/api/api_user/${user.id}/password`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ current_password: update.old_password, new_password: update.new_password1 }),
    });
    if (res.status === 200) {
      passwordForm.reset();
      dispatch({ type: "set_user", user: { ...user, password_enabled: true } });
      notifications.show({
        title: "Updated",
        color: "green",
        message: "",
      });
    } else if (res.status === 400) {
      passwordForm.setFieldError("old_password", "Wrong password");
    } else {
      errorNotification("Something went wrong");
    }
  };

  const removePassword = async (update: PasswordForm) => {
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

    const res = await fetch(`/api/api_user/${user.id}/password`, {
      method: "DELETE",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ current_password: update.old_password }),
    })
    if (res.status === 200) {
      passwordForm.reset();
      dispatch({ type: "set_user", user: { ...user, password_enabled: false } });
      notifications.show({
        title: "Removed Password login",
        color: "green",
        message: "",
      });
    } else if (res.status === 400) {
      passwordForm.setFieldError("old_password", "Wrong password");
    } else {
      errorNotification("Something went wrong");
    }
  };

  return (
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
            <Button type="submit" fullWidth={true} loading={passwordForm.submitting}>
              {user.password_enabled ? "Update Password" : "Create Password"}
            </Button>
          </Grid.Col>
        </Grid>
      </Stack>
    </form>
  );
}
