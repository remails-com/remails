import { NavLink } from "@mantine/core";
import { IconBuildings, IconChartBar, IconGavel, IconServer, IconSettings, IconWorldWww } from "@tabler/icons-react";
import { useRemails } from "../hooks/useRemails.ts";
import { useDisclosure } from "@mantine/hooks";
import { NewOrganization } from "../components/organizations/NewOrganization.tsx";
import OrgDropDown from "./OrgDropDown.tsx";

export function NavBar({ close }: { close: () => void }) {
  const {
    state: { routerState, user },
    navigate,
  } = useRemails();
  const [openedNewOrg, { open: openNewOrg, close: closeNewOrg }] = useDisclosure(false);

  if (!user) {
    return null;
  }

  return (
    <>
      {user.global_role === "admin" && (
        <NavLink
          mb="md"
          label="Organizations"
          active={routerState.name === "organizations"}
          leftSection={<IconBuildings size={20} stroke={1.8} />}
          onClick={() => {
            navigate("organizations");
            close();
          }}
        />
      )}

      <NewOrganization
        opened={openedNewOrg}
        close={closeNewOrg}
        done={(newOrg) => {
          navigate("settings", { org_id: newOrg.id });
        }}
      />

      <OrgDropDown openNewOrg={openNewOrg} />

      <NavLink
        mt="md"
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
        active={routerState.name.startsWith("settings")}
        leftSection={<IconSettings size={20} stroke={1.8} />}
        onClick={() => {
          navigate("settings");
          close();
        }}
      />
      {user.global_role === "admin" && (
        <NavLink
          label="Admin"
          active={routerState.name.startsWith("admin")}
          leftSection={<IconGavel size={20} stroke={1.8} />}
          onClick={() => {
            navigate("admin");
            close();
          }}
        />
      )}
    </>
  );
}
