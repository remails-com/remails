import { Badge, Table, Text, Title } from "@mantine/core";
import OrganizationHeader from "./OrganizationHeader.tsx";
import InfoAlert from "../InfoAlert.tsx";
import StyledTable from "../StyledTable.tsx";
import { useSuppressed } from "../../hooks/useEmails.ts";
import { MaintainerActionIcon } from "../RoleButtons.tsx";
import { IconTrash } from "@tabler/icons-react";
import { formatDateTime } from "../../util.ts";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { notifications } from "@mantine/notifications";
import { errorNotification } from "../../notify.tsx";
import { modals } from "@mantine/modals";

export default function Suppressed() {
  const { currentOrganization } = useOrganizations();
  const { suppressed, setSuppressed } = useSuppressed();

  if (!currentOrganization) {
    return null;
  }

  const unsuppress = async (email: string) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}/emails/suppressed/${encodeURIComponent(email)}`, {
      method: "DELETE",
    });
    if (res.status === 200) {
      setSuppressed((suppressed) => suppressed?.filter((suppressed) => suppressed.email_address !== email) ?? []);
      notifications.show({
        title: "Unsuppressed email address",
        message: "Email address was removed from the suppression list",
        color: "green",
      });
    } else {
      errorNotification("Email address could not be unsuppressed");
      console.error(res);
    }
  };

  const confirmUnsuppress = (email: string) => {
    modals.openConfirmModal({
      title: "Please confirm your action",
      children: <Text>Are you sure you want to unsuppress <Badge color="secondary" variant="light" tt="none" size="lg">{email}</Badge>?</Text>,
      labels: { confirm: "Unsuppress", cancel: "Cancel" },
      onCancel: () => { },
      onConfirm: () => unsuppress(email),
    });
  }

  const rows = suppressed?.map((suppressed) => (
    <Table.Tr key={suppressed.email_address}>
      <Table.Td><Badge color="secondary" variant="light" tt="none" size="lg">{suppressed.email_address}</Badge></Table.Td>
      <Table.Td>{formatDateTime(suppressed.retry_after)}</Table.Td>
      <Table.Td align={"right"}>
        <MaintainerActionIcon
          variant="light"
          tooltip="Remove email address from suppression list"
          onClick={() => confirmUnsuppress(suppressed.email_address)}
          size={30}
        >
          <IconTrash />
        </MaintainerActionIcon>
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <OrganizationHeader allowRename />
      <Title order={3} mb="md">
        Email suppression list
      </Title>
      <InfoAlert stateName="suppressed-emails">
        Email addresses are automatically suppressed when delivery fails repeatedly.
        Once an address is suppressed, Remails stops sending emails to it to prevent further failed deliveries.
        Occasionally, Remails retries delivery to a suppressed address to check whether it has become reachable again.
      </InfoAlert>

      <StyledTable
        headers={[
          "Email address",
          { miw: "10rem", children: "Retry after" },
          "",
        ]}
      >
        {rows}
      </StyledTable>
      {(!rows || rows.length == 0) && "Currently there are no suppressed email address within this organization."}
    </>
  );

}
