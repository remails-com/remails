import {NavLink} from "@mantine/core";
import {IconBuildings, IconServer, IconWorldWww,} from "@tabler/icons-react";
import {useUser} from "../hooks/useUser.ts";
import {is_global_admin} from "../util.ts";
import {useRemails} from "../hooks/useRemails.ts";
import {useCurrentOrganization} from "../hooks/useCurrentOrganization.ts";

export function NavBar() {
  const {state: {route, fullName}, navigate} = useRemails();
  const currentOrganization = useCurrentOrganization();
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
      {/*<NavLink*/}
      {/*    label="Credentials"*/}
      {/*    active={route.name === 'credentials'}*/}
      {/*    leftSection={<IconLockPassword size={20} stroke={1.8}/>}*/}
      {/*    onClick={() => navigate('projects.streams.credentials')}*/}
      {/*/>*/}
      {/*<NavLink*/}
      {/*    label="Message log"*/}
      {/*    active={route.name === 'message-log'}*/}
      {/*    leftSection={<IconMail size={20} stroke={1.8}/>}*/}
      {/*    onClick={() => navigate('projects.streams.message-log', {*/}
      {/*      proj_id: "3ba14adf-4de1-4fb6-8c20-50cc2ded5462",*/}
      {/*      stream_id: "85785f4c-9167-4393-bbf2-3c3e21067e4a"*/}
      {/*    })}*/}
      {/*/>*/}

    </>
  );
}