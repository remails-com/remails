import { IconBuildings } from "@tabler/icons-react";
import { useOrganizations } from "../../hooks/useOrganizations";
import Header from "../Header";

export default function OrganizationHeader() {
  const { currentOrganization } = useOrganizations();

  return <Header name={currentOrganization?.name ?? ""} entityType={"Organization"} Icon={IconBuildings} divider />;
}
