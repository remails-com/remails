import { StreamsOverview } from "../streams/StreamsOverview.tsx";
import DomainsOverview from "../domains/DomainsOverview.tsx";
import ProjectSettings from "./ProjectSettings.tsx";
import { IconAccessPoint, IconSettings, IconWorldWww } from "@tabler/icons-react";
import Tabs from "../../layout/Tabs.tsx";

export default function ProjectDetails() {
  return (
    <Tabs
      tabs={[
        {
          name: "Streams",
          icon: <IconAccessPoint size={12} />,
          content: <StreamsOverview />,
        },
        {
          name: "Domains",
          icon: <IconWorldWww size={12} />,
          content: <DomainsOverview projectDomains={true} />,
        },
        {
          name: "Settings",
          icon: <IconSettings size={12} />,
          content: <ProjectSettings />,
          notSoWide: true,
        },
      ]}
    />
  );
}
