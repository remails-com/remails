import { IconAlertTriangle, IconBuildings } from "@tabler/icons-react";
import { useOrganizations, useOrgRole } from "../../hooks/useOrganizations";
import Header from "../Header";
import { ThemeIcon, Tooltip } from "@mantine/core";
import { errorNotification } from "../../notify";
import { notifications } from "@mantine/notifications";
import { useRemails } from "../../hooks/useRemails";

export default function OrganizationHeader({ allowRename }: { allowRename?: boolean }) {
  const { currentOrganization } = useOrganizations();
  const { isAdmin } = useOrgRole();
  const { dispatch } = useRemails();

  if (!currentOrganization) {
    return null;
  }

  const saveRename = allowRename && isAdmin ? async (values: { name: string }) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values),
    });
    if (res.status !== 200) {
      errorNotification("Organization could not be updated");
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
    dispatch({ type: "add_organization", organization: organization });
  } : undefined;


  let blocked_warning = null;

  if (currentOrganization?.block_status != "not_blocked") {
    const warning = "This organization has been blocked by the Remails admins from sending emails";
    blocked_warning = (
      <Tooltip label={warning}>
        <ThemeIcon variant="transparent">
          <IconAlertTriangle />
        </ThemeIcon>
      </Tooltip>
    );
  }

  return (
    <Header
      name={currentOrganization?.name ?? ""}
      entityType={"Organization"}
      Icon={IconBuildings}
      divider
      addendum={blocked_warning}
      saveRename={saveRename}
    />
  );
}
