import {NavLink} from "@mantine/core";
import {IconBuildings, IconChartBar, IconServer, IconSettings, IconWorldWww,} from "@tabler/icons-react";
import {useUser} from "../hooks/useUser.ts";
import {is_global_admin} from "../util.ts";
import {useRemails} from "../hooks/useRemails.ts";
import {useOrganizations} from "../hooks/useOrganizations.ts";

export function NavBar() {
  const {state: {route, fullName}, navigate} = useRemails();
  const currentOrganization = useOrganizations();
  const {roles} = useUser();

  if (!currentOrganization) {
    return <></>
  }

  return (
    <>
      {is_global_admin(roles) &&
          <NavLink
              label="Organizations"
              active={route.name === 'organizations'}
              leftSection={<IconBuildings size={20} stroke={1.8}/>}
              onClick={() => navigate('organizations')}
          />}
      <NavLink
        label="Projects"
        active={fullName.startsWith('projects')}
        leftSection={<IconServer size={20} stroke={1.8}/>}
        onClick={() => navigate('projects')}
      />
      <NavLink
        label="Domains"
        active={fullName.startsWith('domains')}
        leftSection={<IconWorldWww size={20} stroke={1.8}/>}
        onClick={() => navigate('domains')}
      />
      <NavLink
        label="Statistics"
        active={route.name === 'statistics'}
        leftSection={<IconChartBar size={20} stroke={1.8}/>}
        onClick={() => navigate('statistics')}
      />
      <NavLink
        label="Settings"
        active={route.name === 'settings'}
        leftSection={<IconSettings size={20} stroke={1.8}/>}
        onClick={() => navigate('settings')}
      />
    </>
  );
}