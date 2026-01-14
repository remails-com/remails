import ProjectSettings from "./ProjectSettings.tsx";
import { IconKey, IconMessage, IconServer, IconSettings } from "@tabler/icons-react";
import Tabs from "../../layout/Tabs.tsx";
import { useProjects } from "../../hooks/useProjects.ts";
import Header from "../Header.tsx";
import { EmailOverview } from "../emails/EmailOverview.tsx";
import { CredentialsOverview } from "../smtpCredentials/CredentialsOverview.tsx";

export default function ProjectDetails() {
  const { currentProject } = useProjects();

  return (
    <>
      <Header name={currentProject?.name ?? ""} entityType="Project" Icon={IconServer} />

      <Tabs
        tabs={[
          {
            route: "projects.project.emails",
            name: "Emails",
            icon: <IconMessage size={14} />,
            content: <EmailOverview />,
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
