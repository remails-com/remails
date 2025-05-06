import {Button, Flex, Table} from "@mantine/core";
import {Loader} from "../../Loader";
import {formatDateTime} from "../../util";
import {useProjects} from "../../hooks/useProjects.ts";
import {useRemails} from "../../hooks/useRemails.ts";
import {IconEdit, IconPencilPlus} from "@tabler/icons-react";
import {useDisclosure} from "@mantine/hooks";
import {NewProject} from "./NewProject.tsx";


export function ProjectsOverview() {
  const [opened, {open, close}] = useDisclosure(false);

  const {state: {loading}, navigate} = useRemails();
  const {projects} = useProjects();

  if (loading || projects === null) {
    return <Loader/>;
  }

  const rows = projects.map((project) => (
    <Table.Tr key={project.id}>
      <Table.Td>{project.name}</Table.Td>
      <Table.Td>{formatDateTime(project.updated_at)}</Table.Td>
      <Table.Td align={'right'}>
        <Button
          onClick={() => navigate('projects.project', {
            proj_id: project.id,
          })}><IconEdit/></Button>
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <NewProject opened={opened} close={close}/>
      <Flex justify="flex-end">
        <Button onClick={() => open()} leftSection={<IconPencilPlus/>}>New Project</Button>
      </Flex>
      <Table>
        <Table.Thead>
          <Table.Tr>
            <Table.Th>Name</Table.Th>
            <Table.Th>Updated</Table.Th>
            <Table.Th></Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>{rows}</Table.Tbody>
      </Table>
    </>
  );
}
