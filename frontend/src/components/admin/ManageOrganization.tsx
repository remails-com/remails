import { useForm } from "@mantine/form";
import { useRemails } from "../../hooks/useRemails";
import { Organization, OrgBlockStatus } from "../../types";
import { notifications } from "@mantine/notifications";
import { errorNotification } from "../../notify";
import { Button, Group, Modal, Select, Stack, Title } from "@mantine/core";
import { AdminButton } from "../RoleButtons";
import { IconTrash } from "@tabler/icons-react";

const ALL_BLOCK_STATUSES: OrgBlockStatus[] = ["not_blocked", "no_sending", "no_sending_or_receiving"];
export function isValidBlockStatus(value: string): value is OrgBlockStatus {
  return ALL_BLOCK_STATUSES.includes(value as OrgBlockStatus);
}

const blockSelectData: { value: OrgBlockStatus; label: string }[] = ALL_BLOCK_STATUSES.map((status) => ({
  value: status,
  label: status.replaceAll("_", " "),
}));


interface ManageOrganizationProps {
  opened: boolean;
  close: () => void;
  organization: Organization | null;
}

interface FormValues {
  block_status: OrgBlockStatus;
}

export default function ManageOrganization({ opened, close, organization }: ManageOrganizationProps) {
  const { dispatch } = useRemails();

  const form = useForm<FormValues>({
    validateInputOnBlur: true,
    initialValues: {
      block_status: organization?.block_status ?? "not_blocked",
    },
    validate: {
      block_status: (value) => (isValidBlockStatus(value) ? null : "Invalid block status"),
    },
  });

  if (!organization) {
    return null;
  }

  const save = async (values: FormValues) => {
    const res = await fetch(`/api/organizations/${organization.id}/admin`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values.block_status),
    });
    if (res.status !== 200) {
      errorNotification(`Organization ${organization.name} could not be updated`);
      console.error(res);
      return;
    }
    const newOrganization = await res.json();

    notifications.show({
      title: "Organization updated",
      message: "",
      color: "green",
    });
    dispatch({ type: "remove_organization", organizationId: organization.id });
    dispatch({ type: "add_organization", organization: newOrganization });
    form.resetDirty();
  };

  return (
    <Modal
      opened={opened}
      onClose={close}
      title={
        <Title order={3} component="span">
          Manage organization {organization.name}
        </Title>
      }
      size="lg"
      padding="xl"
      onExitTransitionEnd={form.reset}>
      <form onSubmit={form.onSubmit(save)}>
        <Stack gap="md">
          <Select
            label="Organization block status"
            data={blockSelectData}
            value={form.values.block_status}
            error={form.errors.block_status}
            onChange={(value) => value && isValidBlockStatus(value) && form.setFieldValue("block_status", value)}
          />

          <Group mt="md" justify="space-between">
            <AdminButton
              leftSection={<IconTrash />}
              variant="outline"
              tooltip="Delete organization"
              disabled={true} // TODO: implement delete organization
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
