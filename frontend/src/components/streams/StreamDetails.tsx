import { Loader } from "../../Loader.tsx";
import { useStreams } from "../../hooks/useStreams.ts";
import { Tabs } from "@mantine/core";
import { IconKey, IconMessage, IconSettings } from "@tabler/icons-react";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useProjects } from "../../hooks/useProjects.ts";
import { CredentialsOverview } from "../smtpCredentials/CredentialsOverview.tsx";
import { MessageLog } from "../messages/MessageLog.tsx";
import StreamSettings from "./StreamSettings.tsx";
import { useRemails } from "../../hooks/useRemails.ts";

const DEFAULT_TAB = 'messages';

export default function StreamDetails() {
  const { state: { routerState }, navigate } = useRemails();
  const { currentOrganization } = useOrganizations();
  const { currentStream } = useStreams();
  const { currentProject } = useProjects();

  if (!currentStream || !currentOrganization || !currentProject) {
    return <Loader />;
  }

  const setActiveTab = (tab: string | null) =>  {
    navigate(routerState.name, {}, { tab: tab || DEFAULT_TAB });
  }

  return (
    <Tabs defaultValue="gallery" value={routerState.query.tab || DEFAULT_TAB} onChange={setActiveTab}>
      <Tabs.List mb="md">
        <Tabs.Tab size="lg" value="messages" leftSection={<IconMessage size={12} />}>
          Messages
        </Tabs.Tab>
        <Tabs.Tab size="lg" value="credentials" leftSection={<IconKey size={12} />}>
          Credentials
        </Tabs.Tab>
        <Tabs.Tab size="lg" value="settings" leftSection={<IconSettings size={12} />}>
          Settings
        </Tabs.Tab>
      </Tabs.List>

      <Tabs.Panel value="messages">
        <MessageLog />
      </Tabs.Panel>

      <Tabs.Panel value="credentials">
        <CredentialsOverview />
      </Tabs.Panel>

      <Tabs.Panel value="settings">
        <StreamSettings
          currentStream={currentStream}
          currentOrganization={currentOrganization}
          currentProject={currentProject}
        />
      </Tabs.Panel>
    </Tabs>
  );
}
