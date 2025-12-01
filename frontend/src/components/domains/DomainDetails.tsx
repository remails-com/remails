import { IconSettings, IconWorldCheck, IconWorldWww } from "@tabler/icons-react";
import Tabs from "../../layout/Tabs.tsx";
import Header from "../Header.tsx";
import { useDomains } from "../../hooks/useDomains.ts";
import DomainVerification from "./DomainVerification.tsx";
import DomainSettings from "./DomainSettings.tsx";

export default function DomainDetails() {
  const { currentDomain } = useDomains();

  return (
    <>
      <Header name={currentDomain?.domain ?? ""} entityType="Domain" Icon={IconWorldWww} />

      <Tabs
        tabs={[
          {
            route: "domains.domain",
            name: "DNS Verification",
            icon: <IconWorldCheck size={14} />,
            content: <DomainVerification />,
          },
          {
            route: "domains.domain.settings",
            name: "Settings",
            icon: <IconSettings size={14} />,
            content: <DomainSettings />,
            notSoWide: true,
          },
        ]}
      />
    </>
  );
}
