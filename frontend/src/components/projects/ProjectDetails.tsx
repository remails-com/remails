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
          icon: <IconAccessPoint size={14} />,
          content: <StreamsOverview />,
        },
        {
          name: "Domains",
          icon: <IconWorldWww size={14} />,
          content: <DomainsOverview />,
        },
        {
          name: "Settings",
          icon: <IconSettings size={14} />,
          content: <ProjectSettings />,
          notSoWide: true,
        },
      ]}
    />
  );
}
