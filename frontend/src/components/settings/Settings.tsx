import {Button, Flex} from "@mantine/core";
import {IconPencilPlus} from "@tabler/icons-react";
import {useDisclosure} from "@mantine/hooks";
import {NewOrganization} from "../organizations/NewOrganization.tsx";


export function Settings() {
  const [opened, {open, close}] = useDisclosure(false);
  return (
    <>
      <NewOrganization opened={opened} close={close}/>
      <Flex justify="flex-end">
        <Button onClick={() => open()} leftSection={<IconPencilPlus/>}>New Organization</Button>
      </Flex></>
  )
}