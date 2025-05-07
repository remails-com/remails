import {Button, Group, Stack, Text, Tooltip} from "@mantine/core";
import {IconBrandGithub, IconPencilPlus, IconPlugConnectedX} from "@tabler/icons-react";
import {useDisclosure} from "@mantine/hooks";
import {NewOrganization} from "../organizations/NewOrganization.tsx";
import GitHubBadge from "./GitHubBadge.tsx";
import {useUser} from "../../hooks/useUser.ts";


export function Settings() {
  const [opened, {open, close}] = useDisclosure(false);
  const {user, setUser} = useUser()

  const disconnectGithub = () => {
    fetch('/api/login/github', {
      method: 'DELETE',
    }).then(res => {
      if (res.status === 200) {
        res.json().then(user => {
          setUser(user)
        })
      }
    })
  }

  return (
    <Stack align="flex-start">
      <h2>Login mechanisms</h2>
      <Text>GitHub</Text>
      {user.github_id &&
        (<Group>
          <GitHubBadge user_id={user.github_id}/>
          <Tooltip
            label={user.password_enabled ?
              "Remove Github as login method. You can still sign in with the email and password" :
              "Please set up a password first to not loose access to your account"
            }>
            <Button onClick={disconnectGithub}
                    leftSection={<IconPlugConnectedX/>}
                    color="red"
                    disabled={!user.password_enabled}
            >Disconnect</Button></Tooltip>
        </Group>)}
      {!user.github_id &&
          <Button
              size="xl"
              radius="xl"
              leftSection={<IconBrandGithub/>}
              variant="filled"
              color="black"
              component="a"
              href="/api/login/github"
          >
              Connect with Github
          </Button>
      }


      <Text>Email and Password</Text>

      <h2>Organization Settings</h2>

      <NewOrganization opened={opened} close={close}/>
      {/*<Flex justify="flex-end">*/}
      <Button onClick={() => open()} leftSection={<IconPencilPlus/>}>New Organization</Button>
      {/*</Flex></>*/}
    </Stack>
  )
}
