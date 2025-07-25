import { Container, Tabs as MTabs } from "@mantine/core";
import React from "react";
import { useRemails } from "../hooks/useRemails";
import { RouteName } from "../routes";

type Tab = {
  route: RouteName;
  name: string;
  icon: React.ReactNode;
  content: React.JSX.Element;
  notSoWide?: boolean;
};

export default function Tabs({ tabs }: { tabs: Tab[] }) {
  const {
    state: { routerState },
    navigate,
  } = useRemails();

  const default_route = tabs[0].route;

  const tab_route = tabs.find((t) => t.route == routerState.name) ? routerState.name : default_route;

  const setActiveTab = (route: string | null) => {
    navigate((route as RouteName | null) || default_route);
  };

  return (
    <MTabs value={tab_route} onChange={setActiveTab}>
      <MTabs.List mb="md" mx="-lg" px="lg">
        {tabs.map((t) => (
          <MTabs.Tab size="lg" value={t.route} leftSection={t.icon} key={t.route}>
            {t.name}
          </MTabs.Tab>
        ))}
      </MTabs.List>

      {tabs.map((t) => (
        <MTabs.Panel value={t.route} key={t.route}>
          {t.notSoWide ? (
            <Container size="sm" ml="0" pl="0">
              {t.content}
            </Container>
          ) : (
            t.content
          )}
        </MTabs.Panel>
      ))}
    </MTabs>
  );
}
