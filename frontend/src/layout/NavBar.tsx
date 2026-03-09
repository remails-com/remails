import { BoxProps, NavLink as MantineNavLink } from "@mantine/core";
import { IconBuildings, IconChartBar, IconGavel, IconKey, IconMail, IconReceiptEuro, IconServer, IconUsers, IconWorldWww } from "@tabler/icons-react";
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
  const globalRole = user?.global_role || null;

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
          navigate("organization.subscription", { org_id: newOrg.id });
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
        label="Emails"
        route="emails"
        close={close}
        active={routerState.name.startsWith("emails")}
        leftSection={<IconMail size={20} stroke={1.8} />}
      />
      <NavLink
        label="Statistics"
        route="statistics"
        close={close}
        active={routerState.name === "statistics"}
        leftSection={<IconChartBar size={20} stroke={1.8} />}
      />
      <MantineNavLink
        label="Organization"
        leftSection={<IconBuildings size={20} stroke={1.8} />}
        href="#organization"
        defaultOpened={routerState.name.startsWith("organization")}
      >
        <NavLink
          label="Subscription"
          route="organization.subscription"
          close={close}
          active={routerState.name.startsWith("organization.subscription")}
          leftSection={<IconReceiptEuro size={18} stroke={1.8} />}
        />
        <NavLink
          label="Members"
          route="organization.members"
          close={close}
          active={routerState.name.startsWith("organization.members")}
          leftSection={<IconUsers size={18} stroke={1.8} />}
        />
        <NavLink
          label="API Keys"
          route="organization.API keys"
          close={close}
          active={routerState.name.startsWith("organization.API keys")}
          leftSection={<IconKey size={18} stroke={1.8} />}
        />
        {
          globalRole == "admin" &&
          <NavLink
            label="Admin"
            route="organization.admin"
            close={close}
            active={routerState.name.startsWith("organization.admin")}
            leftSection={<IconGavel size={18} stroke={1.8} />}
          />
        }
      </MantineNavLink>
    </>
  );
}
