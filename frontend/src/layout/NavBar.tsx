import {NavLink} from "@mantine/core";
import {IconBuildings, IconLockPassword, IconMail, IconServer, IconWorldWww} from "@tabler/icons-react";
import {useRouter} from "../hooks/useRouter";
import {useUser} from "../hooks/useUser.ts";
import {is_global_admin} from "../util.ts";

export function NavBar() {
  const {route, navigate} = useRouter();
  const {roles} = useUser();

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
        label="Domains"
        active={route.name === 'domains'}
        leftSection={<IconWorldWww size={20} stroke={1.8}/>}
        onClick={() => navigate('domains')}
      />
      <NavLink
        label="Projects"
        active={route.name === 'projects'}
        leftSection={<IconServer size={20} stroke={1.8}/>}
        onClick={() => navigate('projects')}
      />
      <NavLink
        label="Credentials"
        active={route.name === 'credentials'}
        leftSection={<IconLockPassword size={20} stroke={1.8}/>}
        onClick={() => navigate('credentials')}
      />
      <NavLink
        label="Message log"
        active={route.name === 'message-log'}
        leftSection={<IconMail size={20} stroke={1.8}/>}
        onClick={() => navigate('message-log', {
          proj_id: "3ba14adf-4de1-4fb6-8c20-50cc2ded5462",
          stream_id: "85785f4c-9167-4393-bbf2-3c3e21067e4a"
        })}
      />
    </>
  );
}