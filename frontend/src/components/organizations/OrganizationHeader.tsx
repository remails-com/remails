import { IconBuildings } from "@tabler/icons-react";
import { useOrganizations } from "../../hooks/useOrganizations";
import { Breadcrumbs } from "../../layout/Breadcrumbs";
import EntityHeader from "../EntityHeader";

export default function OrganizationHeader() {
  const { currentOrganization } = useOrganizations();

  return (
    <>
      <EntityHeader name={currentOrganization?.name ?? ""} entityType={"Organization"} Icon={IconBuildings} divider />
      <Breadcrumbs />
    </>
  );
}
