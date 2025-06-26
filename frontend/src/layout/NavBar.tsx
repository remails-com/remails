import { NavLink } from "@mantine/core";
import { IconBuildings, IconChartBar, IconServer, IconSettings, IconWorldWww } from "@tabler/icons-react";
import { useUser } from "../hooks/useUser.ts";
import { is_global_admin } from "../util.ts";
import { useRemails } from "../hooks/useRemails.ts";

export function NavBar({ close }: { close: () => void }) {
  const {
    state: { routerState },
    navigate,
  } = useRemails();
  const {
    user: { roles },
  } = useUser();

  return (
    <>
      {is_global_admin(roles) && (
        <NavLink
          label="Organizations"
          active={routerState.name === "organizations"}
          leftSection={<IconBuildings size={20} stroke={1.8} />}
          onClick={() => {
            navigate("organizations");
            close();
          }}
        />
      )}
      <NavLink
        label="Projects"
        active={routerState.name.startsWith("projects")}
        leftSection={<IconServer size={20} stroke={1.8} />}
        onClick={() => {
          navigate("projects");
          close();
        }}
      />
      <NavLink
        label="Domains"
        active={routerState.name.startsWith("domains")}
        leftSection={<IconWorldWww size={20} stroke={1.8} />}
        onClick={() => {
          navigate("domains");
          close();
        }}
      />
      <NavLink
        label="Statistics"
        active={routerState.name === "statistics"}
        leftSection={<IconChartBar size={20} stroke={1.8} />}
        onClick={() => {
          navigate("statistics");
          close();
        }}
      />
      <NavLink
        label="Settings"
        active={routerState.name === "settings"}
        leftSection={<IconSettings size={20} stroke={1.8} />}
        onClick={() => {
          navigate("settings");
          close();
        }}
      />
    </>
  );
}
