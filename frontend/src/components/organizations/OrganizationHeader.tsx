import { IconAlertTriangle, IconBuildings } from "@tabler/icons-react";
import { useOrganizations } from "../../hooks/useOrganizations";
import Header from "../Header";
import { ThemeIcon, Tooltip } from "@mantine/core";

export default function OrganizationHeader() {
  const { currentOrganization } = useOrganizations();

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
    />
  );
}
