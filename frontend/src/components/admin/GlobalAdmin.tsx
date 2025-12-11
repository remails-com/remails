import Header from "../Header.tsx";
import Tabs from "../../layout/Tabs.tsx";
import { IconBuildings, IconGavel, IconSettings, IconUser } from "@tabler/icons-react";
import OrganizationsOverview from "./OrganizationsOverview.tsx";
import RuntimeConfig from "./RuntimeConfig.tsx";
import ApiUserOverview from "./ApiUserOverview.tsx";

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
          {
            route: "admin.api_users",
            name: "Users",
            icon: <IconUser size={14} />,
            content: <ApiUserOverview />,
          },
        ]}
      />
    </>
  );
}
