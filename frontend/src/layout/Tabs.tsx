import React from "react";
import { useRemails } from "../hooks/useRemails";
import { Tabs as MTabs } from "@mantine/core";

type Tab = {
  name: string;
  icon: React.ReactNode;
  content: React.JSX.Element;
};

export default function Tabs({ tabs }: { tabs: Tab[] }) {
  const {
    state: { routerState },
    navigate,
  } = useRemails();

  const default_tab = tabs[0].name;

  const setActiveTab = (tab: string | null) => {
    navigate(routerState.name, { tab: tab || default_tab });
  };

  return (
    <MTabs defaultValue={default_tab} value={routerState.params.tab || default_tab} onChange={setActiveTab}>
      <MTabs.List mb="md">
        {tabs.map((t) => (
          <MTabs.Tab size="lg" value={t.name} leftSection={t.icon}>
            {t.name}
          </MTabs.Tab>
        ))}
      </MTabs.List>

      {tabs.map((t) => (
        <MTabs.Panel value={t.name}>{t.content}</MTabs.Panel>
      ))}
    </MTabs>
  );
}
