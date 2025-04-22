import { Table } from "@mantine/core";
import { Loader } from "../../Loader";
import { formatDateTime } from "../../util";
import {useProjects} from "../../hooks/useProjects.ts";

export function ProjectsOverview() {
  const { projects } = useProjects();

  if (!projects) {
    return <Loader />;
  }

  const rows = projects.map((project) => (
    <Table.Tr key={project.id}>
      <Table.Td>{project.name}</Table.Td>
      <Table.Td>{formatDateTime(project.updated_at)}</Table.Td>
    </Table.Tr>
  ));

  return (
    <Table>
      <Table.Thead>
        <Table.Tr>
          <Table.Th>Name</Table.Th>
          <Table.Th>Updated</Table.Th>
        </Table.Tr>
      </Table.Thead>
      <Table.Tbody>{rows}</Table.Tbody>
    </Table>
  );
}