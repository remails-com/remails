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
          route: "projects.project.streams.stream.messages",
          name: "Messages",
          icon: <IconMessage size={14} />,
          content: <MessageLog />,
        },
        {
          route: "projects.project.streams.stream.credentials",
          name: "Credentials",
          icon: <IconKey size={14} />,
          content: <CredentialsOverview />,
        },
        {
          route: "projects.project.streams.stream.settings",
          name: "Settings",
          icon: <IconSettings size={14} />,
          content: <StreamSettings />,
          notSoWide: true,
        },
      ]}
    />
  );
}
