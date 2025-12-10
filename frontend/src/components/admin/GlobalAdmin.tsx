import Header from "../Header.tsx";
import Tabs from "../../layout/Tabs.tsx";
import { IconBuildings, IconGavel, IconSettings } from "@tabler/icons-react";
import OrganizationsOverview from "./OrganizationsOverview.tsx";
import RuntimeConfig from "./RuntimeConfig.tsx";

export default function GlobalAdmin() {
  return (
    <>
      <Header name="Settings" entityType="Admin" Icon={IconGavel} />
      <Tabs
        tabs={[
          {
            route: "admin",
            name: "Config",
            icon: <IconSettings size={14} />,
            content: <RuntimeConfig />,
            notSoWide: true,
          },
          {
            route: "admin.organizations",
            name: "Organizations",
            icon: <IconBuildings size={14} />,
            content: <OrganizationsOverview />,
          },
        ]}
      />
    </>
  );
}
