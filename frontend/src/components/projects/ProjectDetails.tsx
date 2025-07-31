import { StreamsOverview } from "../streams/StreamsOverview.tsx";
import DomainsOverview from "../domains/DomainsOverview.tsx";
import ProjectSettings from "./ProjectSettings.tsx";
import { IconAccessPoint, IconServer, IconSettings, IconWorldWww } from "@tabler/icons-react";
import Tabs from "../../layout/Tabs.tsx";
import { useProjects } from "../../hooks/useProjects.ts";
import Header from "../Header.tsx";

export default function ProjectDetails() {
  const { currentProject } = useProjects();

  return (
    <>
      <Header name={currentProject?.name ?? ""} entityType="Project" Icon={IconServer} />

      <Tabs
        tabs={[
          {
            route: "projects.project.streams",
            name: "Streams",
            icon: <IconAccessPoint size={14} />,
            content: <StreamsOverview />,
          },
          {
            route: "projects.project.domains",
            name: "Domains",
            icon: <IconWorldWww size={14} />,
            content: <DomainsOverview />,
          },
          {
            route: "projects.project.settings",
            name: "Settings",
            icon: <IconSettings size={14} />,
            content: <ProjectSettings />,
            notSoWide: true,
          },
        ]}
      />
    </>
  );
}
