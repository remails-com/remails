import { StreamsOverview } from "../streams/StreamsOverview.tsx";
import { useProjects } from "../../hooks/useProjects.ts";
import { Loader } from "../../Loader.tsx";
import { Tabs } from "@mantine/core";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import DomainsOverview from "../domains/DomainsOverview.tsx";
import ProjectSettings from "./ProjectSettings.tsx";
import { IconAccessPoint, IconSettings, IconWorldWww } from "@tabler/icons-react";
import { useRemails } from "../../hooks/useRemails.ts";

const DEFAULT_TAB = 'streams';

export default function ProjectDetails() {
  const { state: { routerState }, navigate } = useRemails();
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();

  if (!currentProject || !currentOrganization) {
    return <Loader />;
  }

  const setActiveTab = (tab: string | null) =>  {
    navigate(routerState.name, {}, { tab: tab || DEFAULT_TAB});
  }

  return (
    <Tabs defaultValue="gallery" value={routerState.query.tab || DEFAULT_TAB} onChange={setActiveTab}>
      <Tabs.List mb="md">
        <Tabs.Tab size="lg" value="streams" leftSection={<IconAccessPoint size={12} />}>
          Streams
        </Tabs.Tab>
        <Tabs.Tab size="lg" value="domains" leftSection={<IconWorldWww size={12} />}>
          Domains
        </Tabs.Tab>
        <Tabs.Tab size="lg" value="settings" leftSection={<IconSettings size={12} />}>
          Settings
        </Tabs.Tab>
      </Tabs.List>

      <Tabs.Panel value="streams">
        <StreamsOverview />
      </Tabs.Panel>

      <Tabs.Panel value="domains">
        <DomainsOverview />
      </Tabs.Panel>

      <Tabs.Panel value="settings">
        <ProjectSettings
          currentOrganization={currentOrganization}
          currentProject={currentProject}
        />
      </Tabs.Panel>
    </Tabs>
  );
}
