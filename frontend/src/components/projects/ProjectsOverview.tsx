import {Button, Table} from "@mantine/core";
import {Loader} from "../../Loader";
import {formatDateTime} from "../../util";
import {useProjects} from "../../hooks/useProjects.ts";
import {useRemails} from "../../hooks/useRemails.ts";
import {IconEdit} from "@tabler/icons-react";

export function ProjectsOverview() {
  const {state: {loading}, navigate} = useRemails();
  const {projects} = useProjects();

  if (loading) {
    return <Loader/>;
  }

  const rows = projects.map((project) => (
    <Table.Tr key={project.id}>
      <Table.Td>{project.name}</Table.Td>
      <Table.Td>{formatDateTime(project.updated_at)}</Table.Td>
      <Table.Td><Button
        onClick={() => navigate('projects.project', {proj_id: project.id})}><IconEdit/></Button></Table.Td>
    </Table.Tr>
  ));

  return (
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
  );
}