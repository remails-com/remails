import { Button, Checkbox, Group, List, Modal, Select, Stack, Text, Title } from "@mantine/core";
import { Role, User } from "../../types.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { errorNotification } from "../../notify.tsx";
import { useForm } from "@mantine/form";
import { IconTrash } from "@tabler/icons-react";
import { notifications } from "@mantine/notifications";
import { AdminButton } from "../RoleButtons.tsx";
import { modals } from "@mantine/modals";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { ROLE_LABELS } from "../../util.ts";

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
  const { organizations } = useOrganizations();
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

  const confirmDeleteUser = (user: User) => {
    modals.openConfirmModal({
      title: `Are you sure you want to delete user ${user.name}?`,
      centered: true,
      children: (
        <>
          <Text>
            The user is in {user.org_roles.length} organization{user.org_roles.length === 1 ? "" : "s"}{user.org_roles.length > 0 ? ":" : "."}
          </Text>
          <List>
            {user.org_roles.map((org) => (
              <List.Item key={org.org_id}>
                {organizations.find((o) => o.id === org.org_id)?.name || org.org_id} ({ROLE_LABELS[org.role]})
              </List.Item>
            ))}
          </List>
          <Text mt="md">
            This action cannot be undone.
          </Text>
        </>
      ),
      labels: { confirm: "Delete", cancel: "Cancel" },
      confirmProps: { color: "red" },
      onConfirm: async () => {
        const res = await fetch(`/api/api_user/${user.id}`, {
          method: "DELETE",
        });
        if (res.ok) {
          dispatch({ type: "remove_api_user", user_id: user.id });
          notifications.show({
            title: "User deleted",
            message: "",
            color: "green",
          });
          close();
        } else {
          errorNotification("Could not delete user");
          console.error(res);
        }
      },
    });
  };

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
              onClick={() => confirmDeleteUser(user)}
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
