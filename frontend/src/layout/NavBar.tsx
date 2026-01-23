import { BoxProps, NavLink as MantineNavLink } from "@mantine/core";
import { IconChartBar, IconGavel, IconServer, IconSettings, IconWorldWww } from "@tabler/icons-react";
import { useRemails } from "../hooks/useRemails.ts";
import { useDisclosure } from "@mantine/hooks";
import { NewOrganization } from "../components/organizations/NewOrganization.tsx";
import OrgDropDown from "./OrgDropDown.tsx";
import { RouteName } from "../routes.ts";

interface NavLinkProps {
  label: string;
  route: RouteName;
  active: boolean;
  close: () => void;
  leftSection?: React.ReactNode;
  style?: BoxProps;
}

function NavLink({ label, route, active, close, leftSection, style }: NavLinkProps) {
  const { navigate, routeToPath } = useRemails();

  return (
    <MantineNavLink
      label={label}
      active={active}
      leftSection={leftSection}
      href={routeToPath(route)}
      onClick={(e) => {
        if (e.defaultPrevented || e.ctrlKey || e.metaKey) {
          return;
        }

        e.preventDefault();
        navigate(route);
        close();
      }}
      {...style}
    />
  );
}

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
          label="Admin"
          active={routerState.name.startsWith("admin")}
          route="admin"
          close={close}
          leftSection={<IconGavel size={20} stroke={1.8} />}
          style={{ mb: "md" }}
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
        label="Projects"
        route="projects"
        close={close}
        active={routerState.name.startsWith("projects")}
        leftSection={<IconServer size={20} stroke={1.8} />}
        style={{ mt: "md" }}
      />
      <NavLink
        label="Domains"
        route="domains"
        close={close}
        active={routerState.name.startsWith("domains")}
        leftSection={<IconWorldWww size={20} stroke={1.8} />}
      />
      <NavLink
        label="Statistics"
        route="statistics"
        close={close}
        active={routerState.name === "statistics"}
        leftSection={<IconChartBar size={20} stroke={1.8} />}
      />
      <NavLink
        label="Settings"
        route="settings"
        close={close}
        active={routerState.name.startsWith("settings")}
        leftSection={<IconSettings size={20} stroke={1.8} />}
      />
    </>
  );
}
