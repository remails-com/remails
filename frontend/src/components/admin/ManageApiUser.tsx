import { Button, Checkbox, Group, Modal, Select, Stack, Title } from "@mantine/core";
import { Role, User } from "../../types.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { errorNotification } from "../../notify.tsx";
import { useForm } from "@mantine/form";
import { IconTrash } from "@tabler/icons-react";
import { notifications } from "@mantine/notifications";
import { AdminButton } from "../RoleButtons.tsx";

interface ManageApiUserProps {
  opened: boolean;
  close: () => void;
  user: User | null;
}

interface FormValues {
  global_role: Role | null;
  blocked: boolean;
}

export default function ManageApiUser({ opened, close, user }: ManageApiUserProps) {
  const { dispatch } = useRemails();
  const {
    state: { user: me },
  } = useRemails();

  const form = useForm<FormValues>({
    initialValues: {
      global_role: user?.global_role || null,
      blocked: user?.blocked || false,
    }
  });

  if (!user) {
    return null;
  }

  const save = async (values: FormValues) => {
    const res = await fetch(`/api/api_user/${user.id}/manage`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(values),
    });
    if (res.ok) {
      dispatch({ type: "update_api_user", user_id: user.id, user: { ...user, ...values } });
      notifications.show({
        title: "User updated",
        message: "",
        color: "green",
      });
      form.resetDirty();
    } else {
      errorNotification("Could not update user");
      console.error(res);
    }
  };

  return (
    <Modal
      opened={opened}
      onClose={close}
      title={
        <Title order={3} component="span">
          Manage user {user.name}
        </Title>
      }
      size="lg"
      padding="xl"
      onExitTransitionEnd={form.reset}
    >
      <form onSubmit={form.onSubmit(save)}>
        <Stack gap="md">
          <Select
            placeholder="none"
            label="Global role"
            size="xs"
            disabled={user.id === me?.id}
            data={["admin"] as Role[]}
            clearable
            value={form.values.global_role}
            onChange={(value) => form.setFieldValue("global_role", value as Role | null)}
          />
          <Checkbox
            label="Block user from accessing Remails"
            disabled={user.id === me?.id}
            checked={form.values.blocked}
            onChange={(e) => form.setFieldValue("blocked", e.currentTarget.checked)}
          />

          <Group mt="md" justify="space-between">
            <AdminButton
              leftSection={<IconTrash />}
              variant="outline"
              tooltip="Delete user"
              disabled={user.id === me?.id}
            // TODO: onClick={() => confirmDeleteProject(currentProject)}
            >
              Delete
            </AdminButton>
            <Group>
              <Button variant="outline" onClick={close}>
                Cancel
              </Button>
              <AdminButton type="submit" disabled={!form.isDirty()} loading={form.submitting}>
                Save
              </AdminButton>
            </Group>
          </Group>

        </Stack>
      </form>
    </Modal>
  );
}
