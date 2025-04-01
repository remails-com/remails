import { NavLink } from "@mantine/core";
import { IconBuildings, IconLockPassword, IconMail, IconWorldWww } from "@tabler/icons-react";
import { useRouter } from "../hooks/useRouter";

export function Menu() {
  const { route, navigate } = useRouter();

  return (
    <>
      <NavLink
        label="Organizations"
        active={route.name === 'organizations'}
        leftSection={<IconBuildings size={16} stroke={1.5} />}
        onClick={() => navigate('organizations')}
      />
      <NavLink
        label="Domains"
        active={route.name === 'domains'}
        leftSection={<IconWorldWww size={16} stroke={1.5} />}
        onClick={() => navigate('domains')}
      />
      <NavLink
        label="Credentials"
        active={route.name === 'credentials'}
        leftSection={<IconLockPassword size={16} stroke={1.5} />}
        onClick={() => navigate('credentials')}
      />
      <NavLink
        label="Message log"
        active={route.name === 'message-log'}
        leftSection={<IconMail size={16} stroke={1.5} />}
        onClick={() => navigate('message-log')}
      />
    </>
  );
}