import { IconKey, IconMessage, IconSettings } from "@tabler/icons-react";
import { CredentialsOverview } from "../smtpCredentials/CredentialsOverview.tsx";
import { MessageLog } from "../messages/MessageLog.tsx";
import StreamSettings from "./StreamSettings.tsx";
import Tabs from "../../layout/Tabs.tsx";

export default function StreamDetails() {
  return (
    <Tabs
      tabs={[
        {
          name: "Messages",
          icon: <IconMessage size={12} />,
          content: <MessageLog />,
        },
        {
          name: "Credentials",
          icon: <IconKey size={12} />,
          content: <CredentialsOverview />,
        },
        {
          name: "Settings",
          icon: <IconSettings size={12} />,
          content: <StreamSettings />,
          notSoWide: true,
        },
      ]}
    />
  );
}
