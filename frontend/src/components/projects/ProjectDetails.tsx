import DomainsOverview from "../domains/DomainsOverview.tsx";
import ProjectSettings from "./ProjectSettings.tsx";
import { IconKey, IconMessage, IconServer, IconSettings, IconWorldWww } from "@tabler/icons-react";
import Tabs from "../../layout/Tabs.tsx";
import { useProjects } from "../../hooks/useProjects.ts";
import Header from "../Header.tsx";
import { MessageLog } from "../messages/MessageLog.tsx";
import { CredentialsOverview } from "../smtpCredentials/CredentialsOverview.tsx";

export default function ProjectDetails() {
  const { currentProject } = useProjects();

  return (
    <>
      <Header name={currentProject?.name ?? ""} entityType="Project" Icon={IconServer} />

      <Tabs
        tabs={[
          {
            route: "projects.project.messages",
            name: "Messages",
            icon: <IconMessage size={14} />,
            content: <MessageLog />,
          },
          {
            route: "projects.project.domains",
            name: "Domains",
            icon: <IconWorldWww size={14} />,
            content: <DomainsOverview />,
          },
          {
            route: "projects.project.credentials",
            name: "Credentials",
            icon: <IconKey size={14} />,
            content: <CredentialsOverview />,
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
