import { Container, Select, Stack } from "@mantine/core";
import { useOrganizations } from "../../hooks/useOrganizations";
import OrganizationHeader from "../organizations/OrganizationHeader";
import { OrgBlockStatus } from "../../types";
import { useForm } from "@mantine/form";
import { MaintainerButton } from "../RoleButtons";
import { errorNotification } from "../../notify";
import { notifications } from "@mantine/notifications";
import { useRemails } from "../../hooks/useRemails";

const ALL_BLOCK_STATUSES: OrgBlockStatus[] = ["not_blocked", "no_sending", "no_sending_or_receiving"];
export function isValidBlockStatus(value: string): value is OrgBlockStatus {
  return ALL_BLOCK_STATUSES.includes(value as OrgBlockStatus);
}

const blockSelectData: { value: OrgBlockStatus; label: string }[] = ALL_BLOCK_STATUSES.map((status) => ({
  value: status,
  label: status.replaceAll("_", " "),
}));

interface FormValues {
  block_status: OrgBlockStatus;
}

export default function Admin() {
  const { dispatch } = useRemails();
  const { currentOrganization } = useOrganizations();

  const form = useForm<FormValues>({
    validateInputOnBlur: true,
    initialValues: {
      block_status: currentOrganization?.block_status ?? "not_blocked",
    },
    validate: {
      block_status: (value) => (isValidBlockStatus(value) ? null : "Invalid block status"),
    },
  });

  if (!currentOrganization) {
    return null;
  }

  const save = async (values: FormValues) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}/admin`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values.block_status),
    });
    if (res.status !== 200) {
      errorNotification(`Organization ${currentOrganization.name} could not be updated`);
      console.error(res);
      return;
    }
    const organization = await res.json();

    notifications.show({
      title: "Organization updated",
      message: "",
      color: "green",
    });
    dispatch({ type: "remove_organization", organizationId: currentOrganization.id });
    dispatch({ type: "add_organization", organization });
  };

  return (
    <>
      <OrganizationHeader />

      <Container size="sm" ml="0" pl="0">
        <form onSubmit={form.onSubmit(save)}>
          <Stack>
            <Select
              label="Organization block status"
              data={blockSelectData}
              value={form.values.block_status}
              error={form.errors.block_status}
              onChange={(value) => value && isValidBlockStatus(value) && form.setFieldValue("block_status", value)}
            />
            <MaintainerButton type="submit" disabled={!form.isDirty()} loading={form.submitting}>
              Save
            </MaintainerButton>
          </Stack>
        </form>
      </Container>
    </>
  );
}
